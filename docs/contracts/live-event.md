# Contract: live events (`stead.live.v1`)

**Frozen.** A compatible superset of mazzap/VEIL's `veil.live.v1` —
both schema tags are accepted, and unknown fields are ignored, so a
Meshtastic gateway, drone bridge, or sensor script can feed either
system with the same payload.

`POST /api/events`:

```json
{
  "schema": "stead.live.v1",
  "kind": "data",
  "device_id": "soil-probe-3",
  "label": "Tomato bed probe",
  "observed_at": "2026-07-11T02:00:00Z",
  "position": {"lat": 38.9, "lon": -77.0, "alt_m": 92.0, "accuracy_m": 3.0},
  "data": {"soil_moisture_pct": 41.5, "temperature_f": 68.2},
  "source": {"protocol": "mqtt", "transport": "wifi"}
}
```

| Field | Required | Notes |
|---|---|---|
| `schema` | yes | `stead.live.v1` or `veil.live.v1` |
| `kind` | yes | `position` · `message` · `data` · `status` · `media` · `command` |
| `device_id` | yes | Stable device identifier; entity id becomes `device:<slug>` |
| `observed_at` | yes | RFC 3339 UTC |
| `position` | no | lat/lon degrees (converted to scene-local via `georef.json` when present) |
| `data` | no | Arbitrary flat object → one observation per key |
| `label`, `source` | no | Display name; provenance protocol/transport |

Conversion: each `data` key becomes an observation
`(device:<id>, <key>, <value>)`; `position` becomes a `position`
observation. Provenance `source` = `source.protocol` (default
`"live"`), `observed_at` from the event.
