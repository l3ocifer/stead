//! stead-ha-sync — one-shot Home Assistant → stead registry sync.
//!
//! ```bash
//! HA_WS_URL=ws://homeassistant.local:8123/api/websocket \
//! HA_TOKEN=<long-lived access token> \
//! STEAD_SITE_DIR=~/sites/home stead-ha-sync
//! ```
//!
//! HA areas become zone stubs (draw boundaries in stead afterwards);
//! HA entities in placeable domains become unplaced features with
//! their `ha_entity` binding wired. Idempotent: re-runs never touch
//! anything that already exists.

use stead_core::{Journal, SiteState};
use stead_ha::sync::{fetch_registries, plan_events};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .ok(); // already installed is fine

    let ws_url = std::env::var("HA_WS_URL").map_err(|_| {
        anyhow::anyhow!("set HA_WS_URL, e.g. ws://homeassistant.local:8123/api/websocket")
    })?;
    let token = std::env::var("HA_TOKEN")
        .map_err(|_| anyhow::anyhow!("set HA_TOKEN (long-lived access token)"))?;
    let site_dir =
        std::path::PathBuf::from(std::env::var("STEAD_SITE_DIR").unwrap_or_else(|_| "site".into()));

    let journal_dir = site_dir.join("journal");
    std::fs::create_dir_all(&journal_dir)?;
    let state = SiteState::replay(&journal_dir)?;

    let snapshot = fetch_registries(&ws_url, &token).await?;
    println!(
        "fetched {} areas, {} floors, {} entities, {} devices",
        snapshot.areas.len(),
        snapshot.floors.len(),
        snapshot.entities.len(),
        snapshot.devices.len()
    );

    let at = humantime::format_rfc3339_seconds(std::time::SystemTime::now()).to_string();
    let events = plan_events(&snapshot, &state, &at);
    if events.is_empty() {
        println!("site already up to date — nothing to sync");
        return Ok(());
    }
    let mut journal = Journal::create(&journal_dir, "ha-sync")?;
    for event in &events {
        journal.append(event)?;
    }
    let zones = events
        .iter()
        .filter(|e| matches!(e, stead_core::JournalEvent::UpsertZone { .. }))
        .count();
    println!(
        "journaled {} events ({zones} zone stubs, {} features) → {}",
        events.len(),
        events.len() - zones,
        journal.path().display()
    );
    println!("next: draw zone boundaries (stead zone-add) and place features");
    Ok(())
}
