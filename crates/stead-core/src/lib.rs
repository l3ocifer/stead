//! stead-core: the site model.
//!
//! A **site** is one property — house, yard, garden, outbuildings —
//! expressed in a single local coordinate frame ("scene-local meters":
//! x = east, y = north, z = up, offset from a projected-CRS origin).
//! The frame convention and its serialized `georef.json` are
//! compatible with mazzap/VEIL twins (see docs/prior-art-mazzap.md)
//! so a land twin and a home twin of the same property can share
//! coordinates.
//!
//! Store semantics (adopted from mazzap, adapted to Rust):
//! - append-only journal of domain events is the system of record;
//!   any index/materialization is disposable
//! - entities are never deleted, only retired (and un-retired)
//! - current state = latest observation per (entity, attribute);
//!   every observation carries provenance

pub mod frame;
pub mod geometry;
pub mod id;
pub mod model;
pub mod state;
pub mod store;

pub use frame::{GeoRef, LocalPoint, SiteFrame};
pub use geometry::{point_in_polygon, BBox, Region, RegionSpec};
pub use id::{named_entity_id, positional_entity_id};
pub use model::{
    Anchor, AnchorKind, Binding, BindingKind, Entity, EntityId, EntityKind, Feature, Observation,
    Provenance, Zone, ZoneKind,
};
pub use state::{SiteState, SiteSummary};
pub use store::{Journal, JournalEvent};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("unknown entity: {0}")]
    UnknownEntity(String),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
