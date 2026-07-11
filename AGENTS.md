# AGENTS.md — stead

Contract for AI coding agents working in this repo.

## Identity

- **Repo**: `stead` (Forgejo source of truth, public mirror at `github.com/l3ocifer/stead`)
- **Default branch**: `main`
- **Stack**: Rust (Cargo workspace)
- **License**: MIT OR Apache-2.0 — this is a public open-source project

## Commands

```bash
cargo build --workspace          # build
cargo test --workspace           # tests
cargo clippy --workspace -- -D warnings   # lint
cargo fmt --all                  # format
```

## Map

- `crates/stead-core` — site model: frames, zones, features, bindings, journal store
- `crates/stead-capture` — scan/walkthrough import (glTF, PLY, LAS)
- `crates/stead-ingest` — live events (`stead.live.v1`), MQTT/HTTP adapters
- `crates/stead-ha` — Home Assistant two-way bridge
- `crates/stead-server` — HTTP API + MCP surface
- `crates/stead-cli` — `stead` binary
- `docs/` — architecture, contracts, prior art

## Non-negotiable design rules

1. **The journal is the system of record.** Indexes/exports are
   disposable derivations. Never write state outside a journal event.
2. **Entities are never deleted** — retire/un-retire only. Current
   state = latest observation per (entity, attr). Every observation
   carries provenance (`source/run_id/confidence/observed_at`).
3. **Coordinates are data, not code.** CRS + origin live in the
   serialized `GeoRef` (mazzap-compatible `georef.json`); scene-local
   meters, x = east, y = north, z = up. No CRS constants in code.
4. **Site data is private.** Real coordinates, scans, and imagery of
   any specific home stay gitignored. The repo ships the engine.
5. **Frozen contracts.** Schema/API changes to anything under
   `docs/contracts/` require an explicit stop-and-flag, never a
   silent extension.
6. See `docs/prior-art-mazzap.md` before redesigning the store, the
   region grammar, or the live event schema.

## Conventions

- Conventional Commits. Subject ≤ 72 chars.
- Branches: `feat/<slug>`, `fix/<slug>`, `chore/<slug>`.
- Tests accompany behavior changes. `cargo clippy -D warnings` clean.
- No new workspace dependencies without justification in the PR body.
