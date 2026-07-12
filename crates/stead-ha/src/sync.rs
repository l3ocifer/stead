//! One-shot Home Assistant registry sync: HA areas/floors become
//! stead zone *stubs* (no boundary yet — draw those later), and HA
//! entities in useful domains become *unplaced* features with their
//! `ha_entity` binding already wired. Re-running is idempotent:
//! anything that already exists in the site is left untouched, so a
//! boundary you drew or a feature you placed is never clobbered.

use std::collections::BTreeMap;

use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use stead_core::{
    named_entity_id, Binding, BindingKind, Feature, JournalEvent, LocalPoint, SiteState, Zone,
    ZoneKind,
};
use tokio_tungstenite::tungstenite::Message;

/// Domains worth representing as placeable features. Everything else
/// (automations, scenes, updates, …) has no physical location.
pub const PLACEABLE_DOMAINS: [&str; 12] = [
    "light",
    "switch",
    "sensor",
    "binary_sensor",
    "climate",
    "fan",
    "cover",
    "camera",
    "media_player",
    "lock",
    "vacuum",
    "valve",
];

#[derive(Debug, Deserialize)]
pub struct HaArea {
    pub area_id: String,
    pub name: String,
    #[serde(default)]
    pub floor_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HaFloor {
    pub floor_id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct HaEntityReg {
    pub entity_id: String,
    #[serde(default)]
    pub area_id: Option<String>,
    #[serde(default)]
    pub device_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub original_name: Option<String>,
    #[serde(default)]
    pub disabled_by: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HaDevice {
    pub id: String,
    #[serde(default)]
    pub area_id: Option<String>,
}

#[derive(Debug)]
pub struct RegistrySnapshot {
    pub areas: Vec<HaArea>,
    pub floors: Vec<HaFloor>,
    pub entities: Vec<HaEntityReg>,
    pub devices: Vec<HaDevice>,
}

/// Fetch the four registries over the HA websocket API.
/// `ws_url` looks like `ws://homeassistant.local:8123/api/websocket`.
pub async fn fetch_registries(ws_url: &str, token: &str) -> anyhow::Result<RegistrySnapshot> {
    let (ws, _) = tokio_tungstenite::connect_async(ws_url).await?;
    let (mut sink, mut stream) = ws.split();

    // auth handshake: auth_required -> auth -> auth_ok
    loop {
        let msg = stream
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("websocket closed during auth"))??;
        let value: Value = serde_json::from_str(msg.to_text()?)?;
        match value.get("type").and_then(Value::as_str) {
            Some("auth_required") => {
                sink.send(Message::text(
                    json!({"type": "auth", "access_token": token}).to_string(),
                ))
                .await?;
            }
            Some("auth_ok") => break,
            Some("auth_invalid") => anyhow::bail!(
                "Home Assistant rejected the token: {}",
                value.get("message").and_then(Value::as_str).unwrap_or("")
            ),
            _ => {}
        }
    }

    let mut results: BTreeMap<u64, Value> = BTreeMap::new();
    let commands = [
        (1u64, "config/area_registry/list"),
        (2, "config/floor_registry/list"),
        (3, "config/entity_registry/list"),
        (4, "config/device_registry/list"),
    ];
    for (id, command) in commands {
        sink.send(Message::text(
            json!({"id": id, "type": command}).to_string(),
        ))
        .await?;
    }
    while results.len() < commands.len() {
        let msg = stream
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("websocket closed mid-fetch"))??;
        let value: Value = serde_json::from_str(msg.to_text()?)?;
        if value.get("type").and_then(Value::as_str) == Some("result") {
            let id = value.get("id").and_then(Value::as_u64).unwrap_or(0);
            if value.get("success").and_then(Value::as_bool) != Some(true) {
                anyhow::bail!("HA command {id} failed: {value}");
            }
            results.insert(id, value.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    Ok(RegistrySnapshot {
        areas: serde_json::from_value(results.remove(&1).unwrap_or_default())?,
        floors: serde_json::from_value(results.remove(&2).unwrap_or_default())?,
        entities: serde_json::from_value(results.remove(&3).unwrap_or_default())?,
        devices: serde_json::from_value(results.remove(&4).unwrap_or_default())?,
    })
}

/// Pure planning: what journal events would bring `existing` up to
/// date with the HA registries. Existing entities are never modified
/// (idempotent, non-clobbering), so this is safe to run on a site
/// with hand-drawn boundaries and placed features.
pub fn plan_events(
    snapshot: &RegistrySnapshot,
    existing: &SiteState,
    at: &str,
) -> Vec<JournalEvent> {
    let floor_names: BTreeMap<&str, &str> = snapshot
        .floors
        .iter()
        .map(|f| (f.floor_id.as_str(), f.name.as_str()))
        .collect();
    let device_areas: BTreeMap<&str, &str> = snapshot
        .devices
        .iter()
        .filter_map(|d| Some((d.id.as_str(), d.area_id.as_deref()?)))
        .collect();
    let area_names: BTreeMap<&str, &str> = snapshot
        .areas
        .iter()
        .map(|a| (a.area_id.as_str(), a.name.as_str()))
        .collect();

    let mut events = Vec::new();
    for area in &snapshot.areas {
        let zone_id = named_entity_id("zone", &area.name);
        if existing.entity(&zone_id).is_some() {
            continue;
        }
        events.push(JournalEvent::UpsertZone {
            zone: Zone {
                id: zone_id,
                kind: ZoneKind::Room,
                name: area.name.clone(),
                floor: area
                    .floor_id
                    .as_deref()
                    .map(|f| floor_names.get(f).copied().unwrap_or(f).to_string()),
                boundary: vec![], // stub — draw it in stead later
                tags: vec![format!("ha:area:{}", area.area_id)],
                expires_at: None,
            },
            at: at.to_string(),
        });
    }

    for entity in &snapshot.entities {
        if entity.disabled_by.is_some() {
            continue;
        }
        let Some(domain) = entity.entity_id.split('.').next() else {
            continue;
        };
        if !PLACEABLE_DOMAINS.contains(&domain) {
            continue;
        }
        // area on the entity, else inherited from its device
        let area_id = entity.area_id.as_deref().or_else(|| {
            entity
                .device_id
                .as_deref()
                .and_then(|d| device_areas.get(d).copied())
        });
        let Some(area_id) = area_id else { continue };

        let feature_id = named_entity_id("feature", &entity.entity_id);
        if existing.entity(&feature_id).is_some() {
            continue;
        }
        events.push(JournalEvent::UpsertFeature {
            feature: Feature {
                id: feature_id,
                name: entity.name.clone().or_else(|| entity.original_name.clone()),
                position: LocalPoint {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                tags: vec![
                    "unplaced".into(),
                    format!("ha:area:{area_id}"),
                    format!(
                        "ha:area_name:{}",
                        area_names.get(area_id).copied().unwrap_or(area_id)
                    ),
                ],
                bindings: vec![Binding {
                    kind: BindingKind::HaEntity,
                    external_id: entity.entity_id.clone(),
                }],
            },
            at: at.to_string(),
        });
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> RegistrySnapshot {
        RegistrySnapshot {
            areas: vec![HaArea {
                area_id: "kitchen".into(),
                name: "Kitchen".into(),
                floor_id: Some("main".into()),
            }],
            floors: vec![HaFloor {
                floor_id: "main".into(),
                name: "Main Floor".into(),
            }],
            entities: vec![
                HaEntityReg {
                    entity_id: "light.kitchen_pendant".into(),
                    area_id: Some("kitchen".into()),
                    device_id: None,
                    name: Some("Kitchen pendant".into()),
                    original_name: None,
                    disabled_by: None,
                },
                HaEntityReg {
                    entity_id: "sensor.kitchen_temp".into(),
                    area_id: None,
                    device_id: Some("dev1".into()),
                    name: None,
                    original_name: Some("Kitchen temperature".into()),
                    disabled_by: None,
                },
                HaEntityReg {
                    entity_id: "automation.morning".into(), // not placeable
                    area_id: Some("kitchen".into()),
                    device_id: None,
                    name: None,
                    original_name: None,
                    disabled_by: None,
                },
            ],
            devices: vec![HaDevice {
                id: "dev1".into(),
                area_id: Some("kitchen".into()),
            }],
        }
    }

    #[test]
    fn plans_zone_stubs_and_bound_features() {
        let state = SiteState::default();
        let events = plan_events(&snapshot(), &state, "2026-07-12T00:00:00Z");
        // 1 zone + 2 placeable features (automation excluded)
        assert_eq!(events.len(), 3);
        let zones: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                JournalEvent::UpsertZone { zone, .. } => Some(zone),
                _ => None,
            })
            .collect();
        assert_eq!(zones.len(), 1);
        assert_eq!(zones[0].floor.as_deref(), Some("Main Floor"));
        assert!(zones[0].boundary.is_empty(), "stub zones have no boundary");
    }

    #[test]
    fn sync_is_idempotent_and_non_clobbering() {
        let mut state = SiteState::default();
        for event in plan_events(&snapshot(), &state, "2026-07-12T00:00:00Z") {
            state.apply(&event);
        }
        let again = plan_events(&snapshot(), &state, "2026-07-12T01:00:00Z");
        assert!(again.is_empty(), "second sync plans nothing");
    }
}
