# Deployment

Reference manifests land alongside the v0.1 server. Planned:

- `kubernetes/` — Deployment + Service + PVC for `stead-server`,
  sized to sit next to Home Assistant and an MQTT broker.
- `compose/` — a `docker-compose.yml` for single-host installs.

Until then, `cargo run -p stead-server` serves the HTTP API locally
(`STEAD_BIND` overrides the default `127.0.0.1:4180`).
