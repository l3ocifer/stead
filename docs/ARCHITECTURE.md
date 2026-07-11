# Architecture

stead is a Cargo workspace built around one idea: **the journal is the
site.** Everything else — in-memory state, HTTP responses, exports,
future 3D tiles — is a derivation that can be deleted and rebuilt.

```
 phone scan ──▶ stead-capture ─┐
 drones ──────▶ stead-ingest ──┤   JournalEvent      ┌─▶ stead-ha (HA sync)
 sensors ─────▶ (LiveEvent) ───┼──▶ journal/*.jsonl ─┼─▶ stead-server (HTTP)
 CLI / API ───────────────────┘         │            └─▶ future: MCP, tiles
                                        ▼
                                  SiteState (replay)
```

## Data flow invariants

1. **Append-then-apply.** Every write path appends a `JournalEvent`
   to the journal *first*, then applies it to the in-memory
   `SiteState`. A crash between the two loses nothing: restart
   replays the journal to the identical state.
2. **Never delete.** Entities are retired (`retired_at`) and
   un-retired; the journal itself is append-only at the file level
   (one file per write session, existing files never modified).
3. **Latest-wins reads, full history in the journal.** Current state
   is the latest observation per (entity, attribute). History is a
   replay query, not archaeology.
4. **Provenance everywhere.** Every observation carries
   `source / run_id / confidence / observed_at`, and the API returns
   it — agent answers must be checkable.
5. **Coordinates are data.** The CRS and origin live in `georef.json`
   (mazzap/VEIL-compatible); code never hardcodes a CRS. Scene-local
   meters: x = east, y = north, z = up.
6. **Deterministic identity.** Positional entities hash
   `source|x±0.1m|y±0.1m` (`positional_entity_id`); named entities
   use slugs (`named_entity_id`). Re-importing unchanged data is a
   no-op.

## Crates

| Crate | Role | Status |
|---|---|---|
| `stead-core` | Site model: frames, zones, features, bindings; journal store; `SiteState` replay; geometry + region grammar; deterministic IDs | functional |
| `stead-ingest` | `stead.live.v1` events (superset of `veil.live.v1`), validation, conversion to observations; MQTT adapter | events functional, MQTT next |
| `stead-server` | HTTP API over a site (zones, features, entities, live events, region queries) | functional |
| `stead-cli` | `stead init/describe/observe/zone-add/entities` | functional |
| `stead-ha` | Home Assistant two-way sync (areas→zones, MQTT discovery back) | scaffold |
| `stead-capture` | Scan/mesh import anchored to the site frame | scaffold |

## The region grammar

Every spatial query takes one `region` argument — exactly one of:

```json
{"all": true}
{"bbox": [minx, miny, maxx, maxy]}
{"within_m": 5.0, "point": {"x": 12.0, "y": -3.5}}
{"polygon": [{"x":0,"y":0}, {"x":10,"y":0}, {"x":10,"y":8}]}
```

Errors are structured and list the valid alternatives. This grammar is
shared with [mazzap](https://github.com/zymazza/mazzap) so agents can
query a land twin and a home twin the same way.

## Frozen contracts

Anything under [`docs/contracts/`](contracts/) is a public interface:
the journal event schema, `georef.json`, and the live event schema.
Changing one is a stop-and-flag event — open an issue first, never
silently extend (see `CONTRIBUTING.md`).

## Performance notes

- `SiteState::replay` is O(events); for large sites the planned
  snapshot file (`state.snapshot.json` + journal tail) makes startup
  O(1)-ish. Until then, journals in the tens of thousands of events
  replay in milliseconds.
- Zone lookup is currently a linear scan with innermost-wins;
  an R-tree lands when a real site shows it matters. Don't optimize
  ahead of measurements.
- Journal files will gain gzip (`.jsonl.gz`) once sizes justify it —
  the reader will accept both.
