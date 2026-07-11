use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use clap::{Parser, Subcommand};
use stead_core::{
    GeoRef, Journal, JournalEvent, LocalPoint, Observation, Provenance, SiteState, Zone, ZoneKind,
};

#[derive(Parser)]
#[command(name = "stead", version, about = "Whole-home localized mapping")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize a new site directory (journal + optional georef).
    Init {
        /// Site directory to create.
        path: PathBuf,
        /// Projected working CRS, e.g. EPSG:26918.
        #[arg(long)]
        crs: Option<String>,
        /// Scene origin as "easting,northing" in the projected CRS.
        #[arg(long)]
        origin: Option<String>,
        /// proj4 string for the CRS (viewers convert client-side).
        #[arg(long)]
        proj4: Option<String>,
    },
    /// Replay the journal and print a site summary.
    Describe { path: PathBuf },
    /// Append an observation (latest state = latest observation).
    Observe {
        path: PathBuf,
        /// Entity id, e.g. zone:kitchen or device:soil_probe_3.
        #[arg(long)]
        entity: String,
        /// Attribute name, e.g. temperature_f.
        #[arg(long)]
        attr: String,
        /// JSON value (bare numbers/strings work: 77.2, "open").
        #[arg(long)]
        value: String,
        /// Provenance source label.
        #[arg(long, default_value = "cli")]
        source: String,
    },
    /// Add or update a zone polygon.
    ZoneAdd {
        path: PathBuf,
        /// Zone name; the id is derived (zone:<slug>).
        #[arg(long)]
        name: String,
        /// room|garden_bed|lawn|deck|path|event_space|utility|other
        #[arg(long, default_value = "room")]
        kind: String,
        /// Floor id for indoor zones (e.g. main, upstairs).
        #[arg(long)]
        floor: Option<String>,
        /// Boundary vertices as "x,y x,y x,y" (scene-local meters).
        #[arg(long)]
        polygon: String,
    },
    /// List entities with their latest attributes.
    Entities { path: PathBuf },
}

fn now() -> String {
    humantime::format_rfc3339_seconds(std::time::SystemTime::now()).to_string()
}

fn journal_dir(site: &Path) -> PathBuf {
    site.join("journal")
}

fn append(site: &Path, session: &str, events: &[JournalEvent]) -> anyhow::Result<()> {
    let mut journal = Journal::create(&journal_dir(site), session)?;
    for event in events {
        journal.append(event)?;
    }
    Ok(())
}

fn parse_polygon(spec: &str) -> anyhow::Result<Vec<LocalPoint>> {
    let points = spec
        .split_whitespace()
        .map(|pair| {
            let (x, y) = pair
                .split_once(',')
                .with_context(|| format!("vertex {pair:?} is not x,y"))?;
            Ok(LocalPoint {
                x: x.parse()?,
                y: y.parse()?,
                z: 0.0,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    if points.len() < 3 {
        bail!("polygon needs at least 3 vertices, got {}", points.len());
    }
    Ok(points)
}

fn zone_kind(s: &str) -> anyhow::Result<ZoneKind> {
    Ok(match s {
        "room" => ZoneKind::Room,
        "garden_bed" => ZoneKind::GardenBed,
        "lawn" => ZoneKind::Lawn,
        "deck" => ZoneKind::Deck,
        "path" => ZoneKind::Path,
        "event_space" => ZoneKind::EventSpace,
        "utility" => ZoneKind::Utility,
        "other" => ZoneKind::Other,
        other => bail!(
            "unknown zone kind {other:?}; accepted: room, garden_bed, lawn, \
             deck, path, event_space, utility, other"
        ),
    })
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Init {
            path,
            crs,
            origin,
            proj4,
        } => {
            std::fs::create_dir_all(journal_dir(&path))?;
            match (crs, origin) {
                (Some(crs), Some(origin)) => {
                    let (e, n) = origin
                        .split_once(',')
                        .context("--origin must be \"easting,northing\"")?;
                    let georef = GeoRef {
                        proj4: proj4.unwrap_or_default(),
                        geographic_crs: "EPSG:4326".into(),
                        origin_utm: (e.trim().parse()?, n.trim().parse()?),
                        analysis_crs: crs,
                    };
                    georef.save(&path.join("georef.json"))?;
                    println!("initialized site at {} (georef written)", path.display());
                }
                _ => {
                    println!("initialized site at {}", path.display());
                    println!(
                        "no georef yet — rerun with --crs EPSG:XXXXX --origin E,N \
                         to anchor the site to real coordinates \
                         (compatible with mazzap/VEIL georef.json)"
                    );
                }
            }
        }
        Command::Describe { path } => {
            let state = SiteState::replay(&journal_dir(&path))?;
            let summary = state.summary();
            println!("site: {}", path.display());
            println!(
                "  entities: {} ({} retired)  zones: {}  features: {}  observations: {}",
                summary.entities,
                summary.retired,
                summary.zones,
                summary.features,
                summary.observations
            );
            for zone in state.zones() {
                println!(
                    "  zone {} — {} ({:?}{})",
                    zone.id,
                    zone.name,
                    zone.kind,
                    zone.floor
                        .as_deref()
                        .map(|f| format!(", floor {f}"))
                        .unwrap_or_default()
                );
            }
        }
        Command::Observe {
            path,
            entity,
            attr,
            value,
            source,
        } => {
            let value: serde_json::Value = serde_json::from_str(&value)
                .unwrap_or_else(|_| serde_json::Value::String(value.clone()));
            append(
                &path,
                "cli-observe",
                &[JournalEvent::Observe(Observation {
                    entity_id: entity.clone(),
                    attr: attr.clone(),
                    value,
                    provenance: Provenance {
                        source,
                        run_id: None,
                        confidence: None,
                        observed_at: now(),
                    },
                })],
            )?;
            println!("observed {entity} {attr}");
        }
        Command::ZoneAdd {
            path,
            name,
            kind,
            floor,
            polygon,
        } => {
            let zone = Zone {
                id: stead_core::named_entity_id("zone", &name),
                kind: zone_kind(&kind)?,
                name,
                floor,
                boundary: parse_polygon(&polygon)?,
                tags: vec![],
            };
            let id = zone.id.clone();
            append(
                &path,
                "cli-zone",
                &[JournalEvent::UpsertZone { zone, at: now() }],
            )?;
            println!("upserted {id}");
        }
        Command::Entities { path } => {
            let state = SiteState::replay(&journal_dir(&path))?;
            for entity in state.entities() {
                let retired = entity
                    .retired_at
                    .as_deref()
                    .map(|at| format!(" [retired {at}]"))
                    .unwrap_or_default();
                println!("{}{retired}", entity.id);
                if let Some(attrs) = state.attrs(&entity.id) {
                    for (attr, obs) in attrs {
                        println!(
                            "    {attr} = {} ({} @ {})",
                            obs.value, obs.provenance.source, obs.provenance.observed_at
                        );
                    }
                }
            }
        }
    }
    Ok(())
}
