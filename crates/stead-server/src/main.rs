//! stead-server: the HTTP API over a site's journal-backed state.
//!
//! Every write appends to the journal first, then applies to the
//! in-memory state — the journal stays the system of record and a
//! restart replays to the identical state. Spatial queries use the
//! shared region grammar (`stead_core::RegionSpec`).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::extract::{Path as UrlPath, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use stead_core::{GeoRef, Journal, JournalEvent, LocalPoint, RegionSpec, SiteState, Zone};
use stead_ingest::LiveEvent;
use tokio::sync::Mutex;

struct App {
    /// Journal + state under one lock: a write is append-then-apply,
    /// and the two must never diverge.
    inner: Mutex<Inner>,
    georef: Option<GeoRef>,
}

struct Inner {
    state: SiteState,
    journal: Journal,
}

impl Inner {
    fn write(&mut self, event: JournalEvent) -> stead_core::Result<()> {
        self.journal.append(&event)?;
        self.state.apply(&event);
        Ok(())
    }
}

type ApiError = (StatusCode, Json<Value>);

fn bad_request(msg: impl Into<String>) -> ApiError {
    (StatusCode::BAD_REQUEST, Json(json!({"error": msg.into()})))
}

fn internal(err: impl std::fmt::Display) -> ApiError {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({"error": err.to_string()})),
    )
}

fn now() -> String {
    humantime::format_rfc3339_seconds(std::time::SystemTime::now()).to_string()
}

async fn health() -> Json<Value> {
    Json(json!({
        "service": "stead-server",
        "version": env!("CARGO_PKG_VERSION"),
        "status": "ok",
    }))
}

/// GET /api/site — summary + georef (the describe_place analogue).
async fn site(State(app): State<Arc<App>>) -> Json<Value> {
    let inner = app.inner.lock().await;
    Json(json!({
        "summary": inner.state.summary(),
        "georef": app.georef,
    }))
}

async fn list_zones(State(app): State<Arc<App>>) -> Json<Value> {
    let inner = app.inner.lock().await;
    Json(json!(inner.state.zones().collect::<Vec<_>>()))
}

async fn list_features(State(app): State<Arc<App>>) -> Json<Value> {
    let inner = app.inner.lock().await;
    Json(json!(inner.state.features().collect::<Vec<_>>()))
}

async fn list_entities(State(app): State<Arc<App>>) -> Json<Value> {
    let inner = app.inner.lock().await;
    Json(json!(inner.state.entities().collect::<Vec<_>>()))
}

/// GET /api/entities/{id} — full current state with provenance.
async fn get_entity(
    State(app): State<Arc<App>>,
    UrlPath(id): UrlPath<String>,
) -> Result<Json<Value>, ApiError> {
    let inner = app.inner.lock().await;
    let entity = inner.state.entity(&id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": format!("unknown entity: {id}"),
                "hint": "list ids at /api/entities",
            })),
        )
    })?;
    Ok(Json(json!({
        "entity": entity,
        "attrs": inner.state.attrs(&id),
    })))
}

#[derive(Deserialize)]
struct LocateQuery {
    x: f64,
    y: f64,
    floor: Option<String>,
}

/// GET /api/zones/locate?x=&y=[&floor=] — innermost zone at a point.
async fn locate_zone(State(app): State<Arc<App>>, Query(q): Query<LocateQuery>) -> Json<Value> {
    let inner = app.inner.lock().await;
    let point = LocalPoint {
        x: q.x,
        y: q.y,
        z: 0.0,
    };
    Json(json!({"zone": inner.state.zone_at(point, q.floor.as_deref())}))
}

/// POST /api/zones — upsert a zone (journaled).
async fn post_zone(
    State(app): State<Arc<App>>,
    Json(zone): Json<Zone>,
) -> Result<Json<Value>, ApiError> {
    if zone.boundary.len() < 3 {
        return Err(bad_request("zone boundary needs at least 3 vertices"));
    }
    let id = zone.id.clone();
    let mut inner = app.inner.lock().await;
    inner
        .write(JournalEvent::UpsertZone { zone, at: now() })
        .map_err(internal)?;
    Ok(Json(json!({"upserted": id})))
}

/// POST /api/events — live ingestion (`stead.live.v1` / `veil.live.v1`).
async fn post_event(
    State(app): State<Arc<App>>,
    Json(event): Json<LiveEvent>,
) -> Result<Json<Value>, ApiError> {
    event.validate().map_err(bad_request)?;
    let entity_id = event.entity_id();
    let observations = event.into_observations();
    let mut inner = app.inner.lock().await;
    if inner.state.entity(&entity_id).is_none() {
        inner
            .write(JournalEvent::UpsertEntity(stead_core::Entity {
                id: entity_id.clone(),
                kind: stead_core::EntityKind::Device,
                name: None,
                created_at: now(),
                retired_at: None,
            }))
            .map_err(internal)?;
    }
    let count = observations.len();
    for obs in observations {
        inner.write(JournalEvent::Observe(obs)).map_err(internal)?;
    }
    Ok(Json(json!({"entity": entity_id, "observations": count})))
}

#[derive(Deserialize)]
struct FeatureQuery {
    region: RegionSpec,
}

/// POST /api/query/features {"region": {...}} — region-grammar query.
async fn query_features(
    State(app): State<Arc<App>>,
    Json(q): Json<FeatureQuery>,
) -> Result<Json<Value>, ApiError> {
    let region = q.region.resolve().map_err(bad_request)?;
    let inner = app.inner.lock().await;
    Ok(Json(json!(inner.state.features_in(&region))))
}

fn open_site(site_dir: &Path) -> anyhow::Result<App> {
    let journal_dir = site_dir.join("journal");
    std::fs::create_dir_all(&journal_dir)?;
    let state = SiteState::replay(&journal_dir)?;
    let journal = Journal::create(&journal_dir, "server")?;
    let georef = GeoRef::load(&site_dir.join("georef.json")).ok();
    tracing::info!(
        site = %site_dir.display(),
        entities = state.summary().entities,
        georeferenced = georef.is_some(),
        "site loaded from journal"
    );
    Ok(App {
        inner: Mutex::new(Inner { state, journal }),
        georef,
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let site_dir = PathBuf::from(std::env::var("STEAD_SITE_DIR").unwrap_or_else(|_| "site".into()));
    let app = Arc::new(open_site(&site_dir)?);

    let router = Router::new()
        .route("/healthz", get(health))
        .route("/api/site", get(site))
        .route("/api/zones", get(list_zones).post(post_zone))
        .route("/api/zones/locate", get(locate_zone))
        .route("/api/features", get(list_features))
        .route("/api/entities", get(list_entities))
        .route("/api/entities/{id}", get(get_entity))
        .route("/api/events", post(post_event))
        .route("/api/query/features", post(query_features))
        .with_state(app);

    let addr = std::env::var("STEAD_BIND").unwrap_or_else(|_| "127.0.0.1:4180".into());
    tracing::info!(%addr, "stead-server listening");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, router).await?;
    Ok(())
}
