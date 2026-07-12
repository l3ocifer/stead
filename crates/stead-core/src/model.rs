//! The domain model: entities (zones, features, devices), observations
//! with provenance, and bindings to external systems (Home Assistant).

use serde::{Deserialize, Serialize};

use crate::frame::LocalPoint;

pub type EntityId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    Zone,
    Feature,
    Device,
    Anchor,
    SurveyFeature,
}

/// A physical relocalization point: where an AR device, robot, or
/// positioning service can re-find the site frame. The payload is
/// whatever the positioning system needs (VPS anchor id, QR contents,
/// AprilTag id) — stead stores it opaquely.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anchor {
    /// "anchor:<slug>"
    pub id: EntityId,
    pub name: Option<String>,
    /// Anchor position in scene-local meters.
    pub position: crate::frame::LocalPoint,
    pub kind: AnchorKind,
    /// Opaque positioning payload, keyed by the anchor kind.
    #[serde(default)]
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnchorKind {
    /// Printed QR code at a known position — the zero-dependency baseline.
    QrCode,
    /// Fiducial marker (AprilTag/ArUco) for robots and CV pipelines.
    Fiducial,
    /// Niantic Lightship VPS anchor.
    VpsLightship,
    /// Google ARCore Geospatial anchor (WGS84 + heading).
    ArcoreGeospatial,
    /// Apple ARKit world anchor / RoomPlan reference.
    ArkitWorld,
    /// Manually surveyed point.
    Manual,
}

/// A stored entity. Never deleted: `retired_at` marks disappearance,
/// and a later re-observation un-retires it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: EntityId,
    pub kind: EntityKind,
    pub name: Option<String>,
    pub created_at: String,
    #[serde(default)]
    pub retired_at: Option<String>,
}

/// Zones are the semantic unit: rooms, garden beds, decks, stages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zone {
    pub id: EntityId,
    pub kind: ZoneKind,
    pub name: String,
    /// Floor id for indoor zones; None for outdoor.
    pub floor: Option<String>,
    /// Closed polygon in scene-local meters (first != last; auto-closed).
    /// May be empty for a *stub* zone (e.g. synced from Home Assistant
    /// before anyone draws it); stubs never match spatial queries.
    pub boundary: Vec<LocalPoint>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// RFC 3339 UTC expiry for temporary zones (the event-planning
    /// lens: a stage or seating area that dissolves after the party).
    /// RFC 3339 UTC strings compare lexicographically, so expiry
    /// checks are plain string comparisons.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

impl Zone {
    pub fn is_expired(&self, now: &str) -> bool {
        self.expires_at.as_deref().is_some_and(|e| e <= now)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZoneKind {
    Room,
    GardenBed,
    Lawn,
    Deck,
    Path,
    EventSpace,
    Utility,
    Other,
}

/// A point/volume feature inside the site: a tree, a fan, a sensor
/// mount, a stage, a raised bed corner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    pub id: EntityId,
    pub name: Option<String>,
    pub position: LocalPoint,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub bindings: Vec<Binding>,
}

/// Binding of a feature/zone to an external control-plane object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binding {
    pub kind: BindingKind,
    /// e.g. Home Assistant entity_id ("switch.backyard_fan_3").
    pub external_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingKind {
    HaEntity,
    HaArea,
    MqttTopic,
    Url,
}

/// Provenance carried by every observation — answers must be
/// checkable (mazzap convention).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    pub source: String,
    pub run_id: Option<String>,
    pub confidence: Option<f32>,
    pub observed_at: String,
}

/// One append-only fact: entity + attribute + value + provenance.
/// Current state is the latest observation per (entity, attr).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub entity_id: EntityId,
    pub attr: String,
    pub value: serde_json::Value,
    pub provenance: Provenance,
}
