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
