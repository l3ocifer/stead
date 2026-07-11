# stead

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

## Quick start (development)

```bash
git clone https://github.com/l3ocifer/stead.git
cd stead
cargo build --workspace
cargo run -p stead-cli -- --help
```

## Deployment

stead ships as a single container designed for Kubernetes or Docker
Compose next to Home Assistant and an MQTT broker. Reference manifests
live in [`deploy/`](deploy/).

## Roadmap

- [ ] v0.1 — site model, zone CRUD, HA area sync, GLB serving
- [ ] v0.2 — phone walkthrough import (ARKit/Polycam/WebODM artifacts)
- [ ] v0.3 — MQTT sensor binding + zone climate envelopes
- [ ] v0.4 — drone mission import (orthophoto + mesh georeferencing)
- [ ] v0.5 — event-planning layer (temporary zones, power/lighting plans)
- [ ] v1.0 — stable API, HACS-installable dashboard card

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
