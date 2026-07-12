# Contract: journal events (`stead.journal.v1`)

**Frozen.** The journal is the system of record; anyone's site data
must replay forever. Existing fields never change shape or meaning.
Two kinds of additions are allowed, each requiring a PR that updates
this doc:

1. New variants — readers must ignore unknown `event` values
   gracefully or fail with a clear message.
2. New *optional* fields on existing variants, with serde defaults,
   so old journals (field absent) and old readers (field unknown)
   both keep working.

One event per line (JSONL), one file per write session under
`<site>/journal/NNNNNN-<session>.jsonl`, ordered by filename.
Existing files are never modified.

## Variants

```json
{"event":"upsert_entity","id":"device:soil_probe_1","kind":"device",
 "name":null,"created_at":"2026-07-11T05:36:24Z","retired_at":null}

{"event":"retire_entity","id":"zone:stage","at":"2026-07-11T01:00:00Z"}

{"event":"upsert_zone","at":"2026-07-11T00:00:00Z","zone":{
  "id":"zone:fire_pit","kind":"event_space","name":"Fire Pit",
  "floor":null,"tags":[],
  "boundary":[{"x":40,"y":40,"z":0},{"x":50,"y":40,"z":0},{"x":50,"y":50,"z":0}]}}

{"event":"upsert_feature","at":"2026-07-11T00:00:01Z","feature":{
  "id":"feature:fan_1","name":"Fire pit fan",
  "position":{"x":45,"y":45,"z":0},"tags":["fan"],
  "bindings":[{"kind":"ha_entity","external_id":"switch.backyard_fan_3"}]}}

{"event":"upsert_anchor","at":"2026-07-12T20:00:00Z","anchor":{
  "id":"anchor:front_door_qr","name":"Front door QR",
  "position":{"x":0,"y":12,"z":0},"kind":"qr_code",
  "payload":{"contents":"stead:site=home;anchor=front_door_qr"}}}

{"event":"observe","entity_id":"zone:fire_pit","attr":"temperature_f",
 "value":74.5,"provenance":{"source":"cli","run_id":null,
 "confidence":null,"observed_at":"2026-07-11T05:36:08Z"}}
```

### Optional fields (added after v1 freeze)

- `zone.expires_at` (RFC 3339 UTC, default absent) — temporary zones
  for the event lens. Expired zones stop matching time-aware queries
  (`zone_at` with a `now`) but replay and remain in the journal.
- `upsert_anchor` variant — relocalization anchors (QR, fiducial,
  Lightship VPS, ARCore Geospatial, ARKit world, manual). `payload`
  is opaque to stead; it belongs to the positioning system named in
  `kind`.

## Semantics

- **Retire, never delete.** `retire_entity` marks; a later upsert of
  the same id un-retires. Ids are permanent.
- **Latest wins.** Current state of `(entity, attr)` is the last
  `observe` in replay order. History = all of them.
- **Coordinates** are scene-local meters (x = east, y = north,
  z = up) in the frame defined by `georef.json`.
- **Timestamps** are RFC 3339 UTC strings.
