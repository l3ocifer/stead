//! MQTT ingestion: subscribe to a topic filter and forward every
//! JSON `stead.live.v1` / `veil.live.v1` payload as a [`LiveEvent`].
//!
//! Publish events to `stead/events/<device_id>` (the default filter is
//! `stead/events/#`) and they land in the site journal. Malformed
//! payloads are logged and dropped — a flaky sensor must never wedge
//! the pipeline.

use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use tokio::sync::mpsc;

use crate::LiveEvent;

#[derive(Debug, Clone)]
pub struct MqttConfig {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub topic: String,
    pub client_id: String,
}

impl MqttConfig {
    /// Read `STEAD_MQTT_HOST` (required), `STEAD_MQTT_PORT` (1883),
    /// `STEAD_MQTT_USERNAME`/`STEAD_MQTT_PASSWORD`, and
    /// `STEAD_MQTT_TOPIC` (`stead/events/#`).
    pub fn from_env() -> Option<Self> {
        let host = std::env::var("STEAD_MQTT_HOST").ok()?;
        Some(Self {
            host,
            port: std::env::var("STEAD_MQTT_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(1883),
            username: std::env::var("STEAD_MQTT_USERNAME").ok(),
            password: std::env::var("STEAD_MQTT_PASSWORD").ok(),
            topic: std::env::var("STEAD_MQTT_TOPIC").unwrap_or_else(|_| "stead/events/#".into()),
            client_id: std::env::var("STEAD_MQTT_CLIENT_ID")
                .unwrap_or_else(|_| "stead-ingest".into()),
        })
    }
}

/// Run the subscription loop, sending validated events to `tx` until
/// the receiver drops. Reconnects are rumqttc's responsibility; fatal
/// connection errors return.
pub async fn run(
    config: MqttConfig,
    tx: mpsc::Sender<LiveEvent>,
) -> Result<(), rumqttc::ConnectionError> {
    let mut options = MqttOptions::new(&config.client_id, &config.host, config.port);
    if let (Some(user), Some(pass)) = (&config.username, &config.password) {
        options.set_credentials(user, pass);
    }
    options.set_keep_alive(std::time::Duration::from_secs(30));

    let (client, mut eventloop) = AsyncClient::new(options, 64);
    // Subscribe after (re)connects, not just once.
    let topic = config.topic.clone();
    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Packet::ConnAck(_))) => {
                if let Err(err) = client.subscribe(&topic, QoS::AtLeastOnce).await {
                    tracing::error!(%err, "mqtt subscribe failed");
                }
                tracing::info!(host = %config.host, %topic, "mqtt ingest connected");
            }
            Ok(Event::Incoming(Packet::Publish(publish))) => {
                match parse_payload(&publish.payload) {
                    Ok(event) => {
                        if tx.send(event).await.is_err() {
                            return Ok(()); // receiver gone — shut down
                        }
                    }
                    Err(err) => {
                        tracing::warn!(topic = %publish.topic, %err, "dropped mqtt payload");
                    }
                }
            }
            Ok(_) => {}
            Err(err) => {
                tracing::error!(%err, "mqtt connection error; retrying in 5s");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }
}

/// Parse and validate one payload — pure, so it's testable offline.
pub fn parse_payload(payload: &[u8]) -> Result<LiveEvent, String> {
    let event: LiveEvent =
        serde_json::from_slice(payload).map_err(|e| format!("not a live event: {e}"))?;
    event.validate()?;
    Ok(event)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_parsing_validates() {
        let good = br#"{"schema":"stead.live.v1","kind":"data",
            "device_id":"probe","observed_at":"2026-07-12T00:00:00Z",
            "data":{"temperature_f":70.1}}"#;
        assert!(parse_payload(good).is_ok());
        assert!(parse_payload(b"not json").is_err());
        let wrong_schema = br#"{"schema":"x","kind":"data",
            "device_id":"probe","observed_at":"2026-07-12T00:00:00Z"}"#;
        assert!(parse_payload(wrong_schema)
            .unwrap_err()
            .contains("stead.live.v1"));
    }
}
