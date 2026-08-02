# stead

[![CI](https://github.com/l3ocifer/stead/actions/workflows/ci.yml/badge.svg)](https://github.com/l3ocifer/stead/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

**Whole-home localized mapping for open-source home automation.**

Walk your property with a phone, and stead builds a live, semantically
tagged 3D map of everything you steward — house, garden, yard, and the
sensors and actuators inside them. Drones, air monitors, motion
sensors, smart plugs, and irrigation controllers all plug into the same
spatial model, and the whole thing binds deeply to
[Home Assistant](https://www.home-assistant.io/) so your automations,
dashboards, and agents can reason about *where* things are — not just
what state they're in.

> Status: **early scaffold** (v0.1). The domain model and Home
> Assistant binding land first; capture and drone adapters follow.
> Built in the open — issues and PRs welcome.

## Why

Home automation platforms know your devices as a flat list with an
"area" label. Real homes are spatial: heat rises through a stairwell,
a rain squall makes every outdoor PIR fire at once, the shaded side of
the garden dries slower, and the fan you want is "the one by the fire
pit," not `switch.plug_7`. stead gives your home a shared coordinate
system and makes location a first-class automation primitive.

## What it does

- **Phone-walkthrough capture** — walk the house and yard filming a
  slow pass (or use any ARKit/ARCore/LiDAR scanning app, or
  photogrammetry via [OpenDroneMap](https://opendronemap.org/));
  stead imports the resulting mesh/point cloud and anchors it to a
  local site frame.
- **Pluggable ingestion** — drones (aerial orthophotos and meshes),
  fixed sensors (MQTT), wearables, and manual annotations all write
  into the same spatial store through one adapter trait.
- **Semantic zones** — rooms, garden beds, decks, paths, and event
  spaces are polygons/volumes with tags, not just names. Query "all
  temperature sensors upstairs" or "every switch within 5 m of the
  stage."
- **Deep Home Assistant integration** — two-way: HA areas/floors/
  labels sync into stead zones; stead exposes zone-aware sensors and
  services back to HA (MQTT discovery), and serves the 3D model +
  entity bindings for dashboard cards (e.g.
  [floor3d-card](https://github.com/adizanni/floor3d-card)).
- **Environmental + thermal control** — zone climate envelopes
  (target temp/humidity per living space), computed from whatever
  sensors fall inside each zone, driving whatever actuators serve it
  (HVAC, fans, vents, shades, irrigation).
- **Three lenses, one map** — the same site model serves **home
  automation** (rooms/devices), **agriculture** (beds, soil, sun
  exposure, irrigation), and **event planning** (stages, seating,
  power runs, lighting scenes).

## Architecture

```
                 ┌────────────────────────────────────────┐
  phone scan ───▶│ stead-capture   mesh/pointcloud import │
  drone flight ─▶│ stead-ingest    adapter trait + MQTT   │
  sensors ──────▶│                                        │
                 │ stead-core      site model: frames,    │
                 │                 zones, features,       │
                 │                 bindings, queries      │
                 │                                        │
                 │ stead-ha        HA sync (WS/REST) +    │
                 │                 MQTT discovery         │
                 │ stead-server    axum API + GLB/tiles   │
                 │                 + live state WS        │
                 └────────────────────────────────────────┘
                        │                    │
                 Home Assistant        agents / dashboards
```

| Crate | Role |
|---|---|
| `stead-core` | Domain model: site frame, zones, features, sensor/actuator bindings, spatial queries |
| `stead-capture` | Walkthrough + photogrammetry import (GLB, LAS/LAZ, OBJ) |
| `stead-ingest` | `Ingestor` trait; MQTT source; drone mission adapters |
| `stead-ha` | Home Assistant WebSocket/REST sync + MQTT discovery publisher |
| `stead-server` | HTTP API, model/tile serving, live WebSocket |
| `stead-cli` | `stead` command-line tool |

## Try it in 60 seconds

```bash
git clone https://github.com/l3ocifer/stead.git && cd stead

# 1. create a site anchored to real coordinates (UTM zone + origin)
cargo run -p stead-cli -- init ~/sites/home --crs EPSG:26918 --origin 322500,4308000

# 2. draw a zone and record an observation
cargo run -p stead-cli -- zone-add ~/sites/home --name "Fire Pit" \
    --kind event_space --polygon "40,40 50,40 50,50 40,50"
cargo run -p stead-cli -- observe ~/sites/home \
    --entity zone:fire_pit --attr temperature_f --value 74.5

# 3. inspect — latest state with provenance, rebuilt from the journal
cargo run -p stead-cli -- describe ~/sites/home
cargo run -p stead-cli -- entities ~/sites/home

# 4. serve the HTTP API
STEAD_SITE_DIR=~/sites/home cargo run -p stead-server

# 5. (optional) let agents query the site over MCP (stdio)
cargo run -p stead-cli -- mcp ~/sites/home
```

Have Home Assistant? Pull your whole registry in — areas become zone
stubs, entities become features with bindings already wired:

```bash
HA_WS_URL=ws://homeassistant.local:8123/api/websocket \
HA_TOKEN=<long-lived token> \
STEAD_SITE_DIR=~/sites/home cargo run -p stead-ha --bin stead-ha-sync
```

Then query it:

```bash
curl localhost:4180/api/site
curl "localhost:4180/api/zones/locate?x=45&y=45"          # → Fire Pit

# live sensor ingestion (stead.live.v1 — veil.live.v1 also accepted)
curl -X POST localhost:4180/api/events -H 'content-type: application/json' -d '{
  "schema": "stead.live.v1", "kind": "data",
  "device_id": "soil-probe-1", "observed_at": "2026-07-11T02:00:00Z",
  "data": {"soil_moisture_pct": 38.2}}'

# spatial queries use one region grammar: all | bbox | within_m+point | polygon
curl -X POST localhost:4180/api/query/features -H 'content-type: application/json' \
  -d '{"region": {"within_m": 10, "point": {"x": 45, "y": 45}}}'
```

Sensors can also stream in over MQTT — set `STEAD_MQTT_HOST` (and
optionally `_PORT`/`_USERNAME`/`_PASSWORD`/`_TOPIC`) on the server and
every `stead.live.v1` payload published to `stead/events/#` is
journaled exactly like a POST.

Everything you wrote is an append-only journal under
`~/sites/home/journal/` — delete the process, restart, and the state
replays identically. That journal is the system of record for your
whole property. Details: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

## Deployment

stead ships as a single container designed for Kubernetes or Docker
Compose next to Home Assistant and an MQTT broker. Reference manifests
live in [`deploy/`](deploy/).

## Roadmap

**v0.1 (now)** — journal store with replay, zones/features/devices,
region-grammar spatial queries, live event ingestion (HTTP + MQTT),
relocalization anchors, TTL zones, HA registry sync, MCP server,
CLI + HTTP API.

- [x] **HA bridge (sync half)**: `stead-ha-sync` pulls areas/floors/
      entities into zone stubs + bound features, idempotently
- [x] **MQTT ingestion**: `STEAD_MQTT_HOST` on the server journals
      every `stead.live.v1` payload on `stead/events/#`
- [x] **MCP server**: `stead mcp <site>` — region-grammar tools over
      stdio so agents query the home like mazzap queries the land
- [x] **Anchors**: QR/fiducial/Lightship/ARCore/ARKit relocalization
      points as first-class journaled entities
- [x] **TTL zones**: temporary event-lens zones that expire
- [ ] v0.2 — HA bridge (publish half): zone-aware aggregate sensors
      back into HA via MQTT discovery
- [ ] v0.3 — zone climate envelopes (per-zone targets computed from
      whatever sensors fall inside, with explicit sensor-coverage
      confidence — honest uncertainty)
- [ ] v0.4 — **capture import**: ARKit/Polycam/Scaniverse and WebODM
      artifacts (GLB/PLY) anchored to the site frame; auto-suggested
      zone boundaries from room meshes
- [ ] v0.6 — **viewer + export**: glTF export with semantic sidecar,
      [floor3d-card](https://github.com/adizanni/floor3d-card)
      bindings for HA dashboards; later a WASM in-browser viewer
- [ ] v0.7 — **event-planning lens**: power-run and lighting-scene
      overlays on TTL zones
- [ ] v0.8 — drone mission import (orthophoto + mesh georeferencing),
      shared-frame interop with a mazzap land twin
- [ ] v1.0 — stable API + contracts, `cargo install stead`,
      single-container deploy, HACS card

Beyond v1.0 — AR overlays bound to anchors, Unity/Unreal live
bridges, reality-linked gameplay, and federated overlay sharing:
see [`docs/VISION.md`](docs/VISION.md).

Sibling project: [firmament](https://github.com/l3ocifer/firmament) —
the astronomically accurate night sky, rendered for ceiling
projection. stead knows where your ceilings are and which projector
serves each room; firmament knows what the sky above them looks like.
A stead zone bound to a firmament render is the "sleep under the real
stars, indoors" overlay.

The binding is live as of firmament v0.3, and it's convention, not
glue code: both sides meet in Home Assistant on the same MQTT broker.
`stead-ha` syncs HA areas into zones; a firmament kiosk started with
`--id <area>` (e.g. `--id bedroom`) discovers as a device in that
same room, so zone presence, bedtime routines, and per-room skies
compose with zero extra configuration — see firmament's
[`docs/HOME_ASSISTANT.md`](https://github.com/l3ocifer/firmament/blob/main/docs/HOME_ASSISTANT.md).

Performance track (as sites grow): state snapshots for O(1) startup,
gzip journal sessions, R-tree zone index.

## Contributing

Issues and PRs are welcome on GitHub — see
[`CONTRIBUTING.md`](CONTRIBUTING.md) for setup, the design rules that
keep site data replayable forever, and the current high-value areas.
Frozen public interfaces live in [`docs/contracts/`](docs/contracts/).

## Prior art & interop

stead's store semantics (append-only journal, retire-not-delete,
provenance on every observation), its "coordinates are data" georef
convention, its MCP region grammar, and its live event schema are
adopted from — and stay interoperable with —
[mazzap/VEIL](https://github.com/zymazza/mazzap) (MIT), a
georeferenced land-scale digital-twin engine. mazzap covers the land;
stead covers the home and its devices. Details and adaptation notes:
[docs/prior-art-mazzap.md](docs/prior-art-mazzap.md).

## License

Dual-licensed under [MIT](LICENSE-MIT) or
[Apache-2.0](LICENSE-APACHE), at your option.
