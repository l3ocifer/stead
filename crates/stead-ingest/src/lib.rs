//! stead-ingest: source-neutral live events into the site store.
//!
//! The wire schema `stead.live.v1` is a compatible superset of
//! mazzap/VEIL's `veil.live.v1` (see docs/prior-art-mazzap.md), so a
//! Meshtastic/LoRa gateway or drone bridge can feed either system.

use serde::{Deserialize, Serialize};

/// One live event from any source (sensor, tracker, drone, gateway).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveEvent {
    /// Schema tag: "stead.live.v1" (accepts "veil.live.v1" too).
    pub schema: String,
    /// "position" | "message" | "data" | "status" | "media" | "command"
    pub kind: String,
    pub device_id: String,
    #[serde(default)]
    pub label: Option<String>,
    pub observed_at: String,
    #[serde(default)]
    pub position: Option<Position>,
    /// Arbitrary sensor payload (temperature, humidity, soil moisture…).
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    #[serde(default)]
    pub source: Option<Source>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub lat: f64,
    pub lon: f64,
    #[serde(default)]
    pub alt_m: Option<f64>,
    #[serde(default)]
    pub accuracy_m: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    /// "mqtt" | "meshtastic" | "http" | "ha-websocket" | …
    pub protocol: String,
    #[serde(default)]
    pub transport: Option<String>,
}

/// Adapter trait: every source (MQTT subscriber, HA websocket, drone
/// upload endpoint) normalizes into [`LiveEvent`]s.
pub trait IngestAdapter {
    fn name(&self) -> &str;
}

pub const SCHEMAS: [&str; 2] = ["stead.live.v1", "veil.live.v1"];
pub const KINDS: [&str; 6] = ["position", "message", "data", "status", "media", "command"];

impl LiveEvent {
    /// Structured validation — errors list the valid alternatives
    /// (mazzap convention), never a bare rejection.
    pub fn validate(&self) -> Result<(), String> {
        if !SCHEMAS.contains(&self.schema.as_str()) {
            return Err(format!(
                "unknown schema {:?}; accepted: {SCHEMAS:?}",
                self.schema
            ));
        }
        if !KINDS.contains(&self.kind.as_str()) {
            return Err(format!("unknown kind {:?}; accepted: {KINDS:?}", self.kind));
        }
        if self.device_id.is_empty() {
            return Err("device_id must be non-empty".into());
        }
        Ok(())
    }

    /// The stable entity this event's observations attach to.
    pub fn entity_id(&self) -> String {
        stead_core::named_entity_id("device", &self.device_id)
    }

    /// Convert into journal-ready observations: one per key of the
    /// `data` payload, plus a `position` observation when present.
    pub fn into_observations(self) -> Vec<stead_core::Observation> {
        let provenance = stead_core::Provenance {
            source: self
                .source
                .as_ref()
                .map(|s| s.protocol.clone())
                .unwrap_or_else(|| "live".into()),
            run_id: None,
            confidence: None,
            observed_at: self.observed_at.clone(),
        };
        let entity_id = self.entity_id();
        let mut out = Vec::new();
        if let Some(pos) = &self.position {
            out.push(stead_core::Observation {
                entity_id: entity_id.clone(),
                attr: "position".into(),
                value: serde_json::to_value(pos).expect("position serializes"),
                provenance: provenance.clone(),
            });
        }
        if let Some(serde_json::Value::Object(map)) = self.data {
            for (attr, value) in map {
                out.push(stead_core::Observation {
                    entity_id: entity_id.clone(),
                    attr,
                    value,
                    provenance: provenance.clone(),
                });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_event_converts_to_observations() {
        let event: LiveEvent = serde_json::from_str(
            r#"{
                "schema": "stead.live.v1",
                "kind": "data",
                "device_id": "soil-probe-3",
                "observed_at": "2026-07-11T02:00:00Z",
                "position": {"lat": 38.9, "lon": -77.0, "alt_m": 92.0},
                "data": {"soil_moisture_pct": 41.5, "temperature_f": 68.2},
                "source": {"protocol": "mqtt"}
            }"#,
        )
        .unwrap();
        event.validate().unwrap();
        assert_eq!(event.entity_id(), "device:soil_probe_3");
        let obs = event.into_observations();
        assert_eq!(obs.len(), 3); // position + 2 data keys
        assert!(obs.iter().all(|o| o.provenance.source == "mqtt"));

        let bad: LiveEvent = serde_json::from_str(
            r#"{"schema": "nope.v9", "kind": "data",
                "device_id": "x", "observed_at": "2026-07-11T02:00:00Z"}"#,
        )
        .unwrap();
        assert!(bad.validate().unwrap_err().contains("stead.live.v1"));
    }
}
