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
    SurveyFeature,
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
    pub boundary: Vec<LocalPoint>,
    #[serde(default)]
    pub tags: Vec<String>,
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
