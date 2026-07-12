//! `SiteState`: the materialized view of a site, rebuilt by replaying
//! the journal. The journal is the system of record; this struct is a
//! disposable derivation — delete it, replay, and you get the exact
//! same state.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;

use crate::frame::LocalPoint;
use crate::geometry::Region;
use crate::model::{Anchor, Entity, EntityId, EntityKind, Feature, Observation, Zone};
use crate::store::{Journal, JournalEvent};
use crate::Result;

#[derive(Debug, Default)]
pub struct SiteState {
    entities: BTreeMap<EntityId, Entity>,
    zones: BTreeMap<EntityId, Zone>,
    features: BTreeMap<EntityId, Feature>,
    anchors: BTreeMap<EntityId, Anchor>,
    /// Latest observation per (entity, attribute). Full history stays
    /// in the journal — history is a replay query, not archaeology.
    latest: BTreeMap<EntityId, BTreeMap<String, Observation>>,
    observation_count: usize,
}

#[derive(Debug, Serialize)]
pub struct SiteSummary {
    pub entities: usize,
    pub retired: usize,
    pub zones: usize,
    pub features: usize,
    pub anchors: usize,
    pub observations: usize,
}

impl SiteState {
    /// Rebuild state by replaying every journal session under `dir`.
    pub fn replay(journal_dir: &Path) -> Result<Self> {
        let mut state = Self::default();
        for event in Journal::replay(journal_dir)? {
            state.apply(&event);
        }
        Ok(state)
    }

    /// Apply one event. Used by replay and by live writers (append to
    /// the journal first, then apply — the journal is the record).
    pub fn apply(&mut self, event: &JournalEvent) {
        match event {
            JournalEvent::UpsertEntity(entity) => {
                self.upsert_entity(entity.clone());
            }
            JournalEvent::RetireEntity { id, at } => {
                if let Some(e) = self.entities.get_mut(id) {
                    e.retired_at = Some(at.clone());
                }
            }
            JournalEvent::UpsertZone { zone, at } => {
                self.upsert_entity(Entity {
                    id: zone.id.clone(),
                    kind: EntityKind::Zone,
                    name: Some(zone.name.clone()),
                    created_at: at.clone(),
                    retired_at: None,
                });
                self.zones.insert(zone.id.clone(), zone.clone());
            }
            JournalEvent::UpsertFeature { feature, at } => {
                self.upsert_entity(Entity {
                    id: feature.id.clone(),
                    kind: EntityKind::Feature,
                    name: feature.name.clone(),
                    created_at: at.clone(),
                    retired_at: None,
                });
                self.features.insert(feature.id.clone(), feature.clone());
            }
            JournalEvent::UpsertAnchor { anchor, at } => {
                self.upsert_entity(Entity {
                    id: anchor.id.clone(),
                    kind: EntityKind::Anchor,
                    name: anchor.name.clone(),
                    created_at: at.clone(),
                    retired_at: None,
                });
                self.anchors.insert(anchor.id.clone(), anchor.clone());
            }
            JournalEvent::Observe(obs) => {
                self.latest
                    .entry(obs.entity_id.clone())
                    .or_default()
                    .insert(obs.attr.clone(), obs.clone());
                self.observation_count += 1;
            }
        }
    }

    /// Insert or un-retire; never deletes (mazzap semantics).
    fn upsert_entity(&mut self, entity: Entity) {
        match self.entities.get_mut(&entity.id) {
            Some(existing) => {
                existing.retired_at = None;
                if entity.name.is_some() {
                    existing.name = entity.name;
                }
            }
            None => {
                self.entities.insert(entity.id.clone(), entity);
            }
        }
    }

    pub fn is_alive(&self, id: &str) -> bool {
        self.entities
            .get(id)
            .is_some_and(|e| e.retired_at.is_none())
    }

    pub fn entity(&self, id: &str) -> Option<&Entity> {
        self.entities.get(id)
    }

    pub fn entities(&self) -> impl Iterator<Item = &Entity> {
        self.entities.values()
    }

    pub fn zones(&self) -> impl Iterator<Item = &Zone> {
        self.zones.values().filter(|z| self.is_alive(&z.id))
    }

    pub fn features(&self) -> impl Iterator<Item = &Feature> {
        self.features.values().filter(|f| self.is_alive(&f.id))
    }

    pub fn anchors(&self) -> impl Iterator<Item = &Anchor> {
        self.anchors.values().filter(|a| self.is_alive(&a.id))
    }

    /// Latest observation for one (entity, attribute).
    pub fn latest(&self, id: &str, attr: &str) -> Option<&Observation> {
        self.latest.get(id)?.get(attr)
    }

    /// All current attributes of one entity, with provenance.
    pub fn attrs(&self, id: &str) -> Option<&BTreeMap<String, Observation>> {
        self.latest.get(id)
    }

    /// The innermost living zone containing a point. When `floor` is
    /// given, only zones on that floor match; otherwise any zone does.
    /// When `now` (RFC 3339 UTC) is given, expired temporary zones are
    /// skipped. "Innermost" = smallest bounding box, so a nested zone
    /// (a garden bed inside the yard) wins over its container.
    pub fn zone_at(
        &self,
        point: LocalPoint,
        floor: Option<&str>,
        now: Option<&str>,
    ) -> Option<&Zone> {
        self.zones()
            .filter(|z| now.is_none_or(|n| !z.is_expired(n)))
            .filter(|z| match (floor, z.floor.as_deref()) {
                (Some(f), Some(zf)) => f == zf,
                (Some(_), None) | (None, _) => floor.is_none(),
            })
            .filter(|z| crate::geometry::point_in_polygon(point, &z.boundary))
            .min_by(|a, b| {
                let area = |z: &Zone| {
                    crate::geometry::BBox::of(&z.boundary)
                        .map(|b| (b.max_x - b.min_x) * (b.max_y - b.min_y))
                        .unwrap_or(f64::MAX)
                };
                area(a).total_cmp(&area(b))
            })
    }

    /// Living features whose position falls inside a region.
    pub fn features_in(&self, region: &Region) -> Vec<&Feature> {
        self.features()
            .filter(|f| region.contains(f.position))
            .collect()
    }

    pub fn summary(&self) -> SiteSummary {
        SiteSummary {
            entities: self.entities.len(),
            retired: self
                .entities
                .values()
                .filter(|e| e.retired_at.is_some())
                .count(),
            zones: self.zones.len(),
            features: self.features.len(),
            anchors: self.anchors.len(),
            observations: self.observation_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Provenance;

    fn p(x: f64, y: f64) -> LocalPoint {
        LocalPoint { x, y, z: 0.0 }
    }

    fn zone(id: &str, name: &str, ring: Vec<LocalPoint>) -> JournalEvent {
        JournalEvent::UpsertZone {
            zone: Zone {
                id: id.into(),
                kind: crate::model::ZoneKind::Room,
                name: name.into(),
                floor: None,
                boundary: ring,
                tags: vec![],
                expires_at: None,
            },
            at: "2026-07-11T00:00:00Z".into(),
        }
    }

    #[test]
    fn replay_materializes_zones_features_and_latest_attrs() {
        let mut state = SiteState::default();
        state.apply(&zone(
            "zone:yard",
            "Yard",
            vec![p(0.0, 0.0), p(100.0, 0.0), p(100.0, 100.0), p(0.0, 100.0)],
        ));
        state.apply(&zone(
            "zone:fire_pit",
            "Fire pit",
            vec![p(40.0, 40.0), p(50.0, 40.0), p(50.0, 50.0), p(40.0, 50.0)],
        ));
        state.apply(&JournalEvent::UpsertFeature {
            feature: Feature {
                id: "feature:fan_1".into(),
                name: Some("Fire pit fan".into()),
                position: p(45.0, 45.0),
                tags: vec!["fan".into()],
                bindings: vec![],
            },
            at: "2026-07-11T00:00:01Z".into(),
        });
        for (i, temp) in [71.0, 73.5].iter().enumerate() {
            state.apply(&JournalEvent::Observe(Observation {
                entity_id: "zone:fire_pit".into(),
                attr: "temperature_f".into(),
                value: serde_json::json!(temp),
                provenance: Provenance {
                    source: "test".into(),
                    run_id: None,
                    confidence: None,
                    observed_at: format!("2026-07-11T00:00:0{}Z", i + 2),
                },
            }));
        }

        // innermost zone wins
        let z = state.zone_at(p(45.0, 45.0), None, None).unwrap();
        assert_eq!(z.id, "zone:fire_pit");
        // latest observation, not the first
        let obs = state.latest("zone:fire_pit", "temperature_f").unwrap();
        assert_eq!(obs.value, serde_json::json!(73.5));
        // region query finds the fan
        let region = crate::geometry::RegionSpec {
            within_m: Some(10.0),
            point: Some(p(45.0, 45.0)),
            ..Default::default()
        }
        .resolve()
        .unwrap();
        assert_eq!(state.features_in(&region).len(), 1);
        assert_eq!(state.summary().observations, 2);
    }

    #[test]
    fn retire_and_unretire() {
        let mut state = SiteState::default();
        state.apply(&zone(
            "zone:stage",
            "Stage",
            vec![p(0.0, 0.0), p(5.0, 0.0), p(5.0, 5.0), p(0.0, 5.0)],
        ));
        state.apply(&JournalEvent::RetireEntity {
            id: "zone:stage".into(),
            at: "2026-07-11T01:00:00Z".into(),
        });
        assert!(state.zone_at(p(2.0, 2.0), None, None).is_none());
        assert_eq!(state.summary().retired, 1);
        // reappearing un-retires the same identity
        state.apply(&zone(
            "zone:stage",
            "Stage",
            vec![p(0.0, 0.0), p(5.0, 0.0), p(5.0, 5.0), p(0.0, 5.0)],
        ));
        assert!(state.zone_at(p(2.0, 2.0), None, None).is_some());
        assert_eq!(state.summary().retired, 0);
    }

    #[test]
    fn temporary_zones_expire() {
        let mut state = SiteState::default();
        state.apply(&JournalEvent::UpsertZone {
            zone: Zone {
                id: "zone:party_stage".into(),
                kind: crate::model::ZoneKind::EventSpace,
                name: "Party stage".into(),
                floor: None,
                boundary: vec![p(0.0, 0.0), p(5.0, 0.0), p(5.0, 5.0), p(0.0, 5.0)],
                tags: vec![],
                expires_at: Some("2026-07-12T23:00:00Z".into()),
            },
            at: "2026-07-11T00:00:00Z".into(),
        });
        let during = Some("2026-07-12T20:00:00Z");
        let after = Some("2026-07-13T09:00:00Z");
        assert!(state.zone_at(p(2.0, 2.0), None, during).is_some());
        assert!(state.zone_at(p(2.0, 2.0), None, after).is_none());
        // without a time reference the zone still resolves
        assert!(state.zone_at(p(2.0, 2.0), None, None).is_some());
    }

    #[test]
    fn anchors_materialize() {
        let mut state = SiteState::default();
        state.apply(&JournalEvent::UpsertAnchor {
            anchor: Anchor {
                id: "anchor:front_door_qr".into(),
                name: Some("Front door QR".into()),
                position: p(0.0, 12.0),
                kind: crate::model::AnchorKind::QrCode,
                payload: serde_json::json!({"contents": "stead:site=home;anchor=front_door_qr"}),
            },
            at: "2026-07-12T00:00:00Z".into(),
        });
        assert_eq!(state.anchors().count(), 1);
        assert_eq!(state.summary().anchors, 1);
    }
}
