//! stead-ha: the Home Assistant bridge.
//!
//! Inbound: HA areas/floors/labels and entity registry sync into
//! stead zones and bindings (websocket API). Outbound: stead exposes
//! zone-aware aggregate sensors and services back to HA via MQTT
//! discovery (e.g. `sensor.stead_kitchen_temperature`, a service to
//! actuate "all switches within N meters of a point").

pub mod sync;

use serde::{Deserialize, Serialize};

/// Connection settings for the HA websocket + MQTT discovery paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HaConfig {
    /// e.g. "ws://homeassistant.home-services.svc:8123/api/websocket"
    pub websocket_url: String,
    /// Long-lived access token (injected via env/secret, never stored).
    #[serde(skip_serializing)]
    pub token: String,
    /// MQTT broker for discovery-based outbound sensors.
    pub mqtt_host: Option<String>,
    pub mqtt_port: Option<u16>,
}

/// An HA area as reported by the registry, pre-mapping into a zone.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HaArea {
    pub area_id: String,
    pub name: String,
    pub floor_id: Option<String>,
}
