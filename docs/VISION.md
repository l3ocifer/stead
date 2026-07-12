# Vision: from home map to shared world

stead starts as whole-home localized mapping for open-source home
automation. This document is the longer arc: how the same site model
becomes a substrate for AR overlays, games, and a federated network of
places people actually live in — without ever giving up the thing
that makes it trustworthy: **your site data stays yours.**

## The thesis

Every big spatial-computing platform (Niantic Lightship, Google
ARCore Geospatial, Apple ARKit, Meta Presence) is built outside-in:
a global cloud map of public space, with your home as an afterthought
— or a privacy liability they legally can't touch. stead is built
inside-out: the richest, most private spatial data (your home, your
land, your devices) lives on hardware you control, and *interops
outward* through anchors and published overlays.

That's the needle-thread: **don't compete with global VPS platforms —
be the sovereign private layer they cannot build.** The same position
Home Assistant occupies against Google Home and Alexa. Home Assistant
proved the model: the open, local-first option becomes the center of
gravity for everyone who cares, and eventually for vendors too.

## The layers

```
L4  shared worlds     overlays published + subscribed across sites
L3  experiences       AR walkthroughs, games, event previews
L2  runtime           live state: sensors, occupancy, climate, scenes
L1  semantics         zones, features, bindings, provenance   ← stead today
L0  geometry          meshes, point clouds, floor plans, georef
```

Everything above L1 consumes the same journal + region grammar that
exists now. Nothing in L3/L4 requires redesigning L1 — that's why the
contracts are frozen early.

## Anchors: the bridge to every positioning system

An **anchor** (in the model as of v0.1.x) is a physical
relocalization point: a position in the site frame plus an opaque
payload for whatever system re-finds it.

- **QR / AprilTag** — the zero-dependency baseline. Print a code, tape
  it by the door, scan it with any phone: the device now knows where
  it is in *your* frame. No cloud, no SDK, works forever.
- **Niantic Lightship VPS** — store the VPS anchor id as payload; a
  Lightship app relocalizes against Niantic's mesh, then transforms
  into the stead frame. Useful for yards and street-facing spaces
  that Niantic has mapped.
- **Google ARCore Geospatial** — WGS84 + heading payload; since
  `georef.json` already anchors the site to a projected CRS, the
  transform is closed-form. Best outdoors.
- **Apple ARKit / RoomPlan** — world-anchor payloads for iOS capture
  and walkthrough apps; RoomPlan output doubles as L0 geometry and
  auto-suggested zone boundaries.

The strategic point: anchors make stead *positioning-system neutral*.
Apps choose whatever relocalizes best in context; stead is the frame
they all agree on.

## Game engines: your home as a level

The site model exports cleanly to Unity and Unreal because it is
already engine-shaped: meters, right-handed, semantic objects.

1. **Static export** — `stead export --format gltf` bundles L0 meshes
   with a JSON sidecar of zones/features/bindings (glTF `extras`).
   Any engine, Blender, or three.js loads it. This is the v0.6 viewer
   work generalized.
2. **Live bridge** — the server's WebSocket streams observations;
   an engine plugin subscribes and drives materials/particles from
   real sensor state. Room glows red when it's hot. Rain guard on →
   rain in the game.
3. **Reality-linked gameplay** — the killer demo. Bindings run both
   ways through Home Assistant: a game event can flip a real switch.
   Laser tag where a hit flashes the room you're standing in; a
   haunted-house mode that controls actual lights and fans; a
   kids' treasure hunt where finding the virtual key unlocks the real
   smart lock. Nobody else can ship this, because nobody else has the
   actuator bindings.

Engine adapters live outside the core (a `stead-unity` package, an
Unreal plugin) speaking only the public HTTP/WS API — same rule as
mazzap's engine/content split.

## AR overlays: create on your world, share it

An **overlay** is authored content bound to anchors and zones: garden
plans, art installations, event layouts, game content, guided tours.

- Overlays are **packs**: signed, versioned bundles referencing
  anchor ids and zone semantics — *not* raw geometry. You can share
  "the tomato-bed plan" or "the Halloween walk" without shipping a
  mesh of your house.
- Subscribing to an overlay on someone else's site works because both
  sides speak the same region grammar and anchor model; the overlay
  re-binds to the local site's anchors ("front door," "stage") by
  tag, like a theme re-binds to a website's CSS slots.
- A phone AR client (WebXR first — no app install) renders overlays
  relocalized via any anchor kind above.

## Federation: many steads, one world, no landlord

Long-term, sites federate the way Mastodon instances do:

- Each stead is sovereign — its journal never leaves the site.
- Sites *publish* selected overlays and coarse presence ("event here
  Saturday, RSVP for the AR preview") to relays they choose.
- Community spaces (makerspaces, farms, venues, campgrounds) run
  community steads; an event overlay authored at home deploys onto
  the venue's anchors for the day and expires (TTL zones — already
  shipped).

The "biggest world people live in" isn't one company's map. It's the
sum of places whose stewards mapped them, kept them, and chose what
to share. That is a world you *inhabit*, not one you're inventoried
in.

## Standards we align with (not reinvent)

- **OGC GeoPose** for anchor/device pose interchange.
- **glTF** for geometry; **3D Tiles** if/when sites get big.
- **CityJSON** import for parcel/building context; **Overture Maps**
  for surroundings.
- **MQTT + Home Assistant conventions** for the device plane;
  **Matter** as it matures.
- **MCP** for the agent plane — shipped: `stead mcp <site>`.

## Sequencing (how we don't die of scope)

The wedge stays narrow: be *excellent* for HA users first (v0.2–v0.6
roadmap in the README). Each ring of the vision only unlocks when the
previous one has users: home automation → garden/agriculture → events
→ overlays/AR → games → federation. Anchors ship early (done) because
every later ring needs them and they cost little now; federation
ships last because it costs the most and needs the crowd.

## What we will not do

- No cloud dependency for core function, ever.
- No harvesting site data; telemetry is opt-in and boring.
- No proprietary anchor format — payloads belong to their systems.
- No engine fork-lock: game adapters consume the public API only.
