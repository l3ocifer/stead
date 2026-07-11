# Prior art: mazzap / VEIL

[zymazza/mazzap](https://github.com/zymazza/mazzap) (MIT) is a
region-agnostic, georeferenced 3D digital-twin engine for land: DEM +
aerial imagery + open geospatial layers → a browser-viewable twin with
hydrology/ET/wildfire/solar simulation, an MCP query surface, QField
survey round-trips, and Meshtastic live telemetry. Reviewed 2026-07-10
(v2.0.0). stead deliberately overlaps it very little — mazzap is
outdoor land-scale with no indoor capture, no Home Assistant binding,
and no actuator control — but several of its design decisions are
excellent and stead adopts them explicitly:

## Adopted patterns (with adaptation notes)

1. **Append-only journal as the system of record.** mazzap journals
   every store write (`journal/*.jsonl.gz`) and materializes a
   queryable GeoPackage that is safe to delete and rebuild
   byte-identically. stead: same split — `stead-core` journals domain
   events; the SQLite/spatial index is a disposable materialization.
2. **Entities are never deleted.** Disappearing entities are
   *retired* (and un-retired if they reappear); current state is the
   latest observation per entity+attribute; history is a query, not
   archaeology. stead: identical semantics in `stead-core`.
3. **Provenance on every attribute.** `source / confidence / run_id /
   observed_at` returned with every fact. stead: `Observation` carries
   the same fields; agent answers must be checkable.
4. **Coordinates are data, not code.** One `georef.json` (projected
   CRS EPSG + proj4 string + scene origin); scene-local meters
   (x = east, y = north); no module-level CRS constants anywhere.
   stead: `SiteFrame` serializes to a mazzap-compatible `georef.json`
   so a stead site and a mazzap twin of the same property can share
   coordinates. Indoors adds per-floor local frames anchored to the
   site frame.
5. **Frozen interface contracts.** mazzap freezes its terrain grid
   contract in `docs/grid-contract.md` and treats changes as
   stop-and-flag events. stead: same discipline for the site-model
   schema and HTTP API (`docs/contracts/`).
6. **Engine vs. content packs.** The engine names no CRS, layer, or
   species; regional/domain knowledge is a pack. stead: the engine
   knows zones/features/bindings; domain semantics (garden botany,
   event-planning fixtures, HVAC heuristics) arrive as packs.
7. **MCP region grammar.** Every spatial tool takes one `region`
   argument — `{aoi}` | `{bbox}` | `{within_m, point}` | `{polygon}`
   | visibility shapes; points accept `{lat,lon}` or local `{x,y}`
   and outputs echo both; errors are structured with the valid
   alternatives listed. stead: `stead-server`'s MCP tools use the
   same grammar so agents can move between a mazzap land twin and a
   stead home twin without relearning.
8. **Source-neutral live event schema.** `veil.live.v1` events
   (`position|message|data|status|media|command` + device/motion/
   link/source blocks). stead: `stead-ingest`'s wire event is a
   superset (`stead.live.v1`) that remains translatable 1:1, so a
   Meshtastic gateway or drone bridge can feed both.
9. **Honest uncertainty.** Simulations report geometry as reliable
   and magnitudes with explicit error bands. stead: zone climate
   estimates state sensor coverage and interpolation confidence.

## What we intentionally do differently

- **Rust workspace** (mazzap: Python pipeline + zero-dep Node server).
- **Indoor + outdoor in one model** — rooms/floors with local frames,
  not just terrain.
- **Devices are first-class**: sensor/actuator bindings on features
  and zones, synced two-way with Home Assistant; mazzap is read-only
  observation of land.
- **Control loops** (zone climate envelopes) — stead acts, mazzap
  analyzes.

## Interop opportunities (roadmap)

- Build a mazzap VEIL of the property (US pack: 3DEP/NAIP/LANDFIRE/
  gSSURGO) for yard-scale hydrology, solar siting, and viewshed;
  share its `georef.json` with the stead site so both twins speak the
  same coordinates.
- Register both MCP servers side by side; agents answer "where does
  water pool after a storm" from mazzap and "turn on the fans within
  10 m of the stage" from stead.
- Reuse its QField survey loop for garden-bed mapping until stead's
  phone capture lands.
