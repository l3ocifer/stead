# Contributing to stead

Thanks for helping build open-source spatial home automation. Issues,
PRs, design discussions, and field reports from real homes are all
welcome.

## Where development happens

The canonical repository is self-hosted; **github.com/l3ocifer/stead
is a live mirror and the right place for community issues and PRs.**
Maintainers import merged GitHub PRs into the canonical repo, which
mirrors back — your commits and authorship are preserved. Don't be
surprised by a small delay between merge and mirror sync.

## Dev setup

```bash
git clone https://github.com/l3ocifer/stead.git
cd stead
cargo test --workspace          # everything should pass on stable
cargo clippy --workspace -- -D warnings
cargo fmt --all
```

No system dependencies — pure Rust, stable toolchain. Try the 60-second
demo in the README to see the moving parts.

## What to work on

Check issues labeled `good first issue` and the roadmap in the README.
High-value areas right now:

- **Capture importers** (`stead-capture`): glTF/GLB and PLY parsing,
  transform application, feature extraction from scan artifacts.
- **MQTT adapter** (`stead-ingest`): subscribe, normalize to
  `stead.live.v1`, forward to the journal.
- **HA bridge** (`stead-ha`): websocket area/entity registry sync,
  MQTT discovery publishing.
- **Snapshotting**: `state.snapshot.json` + journal tail for O(1)
  startup on large sites.
- **MCP server**: the region-grammar query tools over HTTP/stdio.

## Design rules (non-negotiable)

These keep every user's site data replayable forever — see
[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the reasoning:

1. The journal is the system of record; writes are append-then-apply.
2. Entities are never deleted, only retired.
3. Every observation carries provenance.
4. Coordinates are data (`georef.json`), never code constants.
5. Anything in [`docs/contracts/`](docs/contracts/) is frozen — open
   an issue *before* a PR that touches a contract.

## PR checklist

- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` is clean
- [ ] `cargo fmt --all` applied
- [ ] Behavior changes come with tests
- [ ] Contract changes were discussed in an issue first
- [ ] Commit messages follow Conventional Commits (`feat:`, `fix:`, …)

## Privacy expectation

Never commit a real site's data (coordinates, scans, imagery,
`georef.json`). Test fixtures must be synthetic. The `.gitignore`
guards the obvious paths; reviewers will flag anything that looks like
a real home.

## Code of conduct

Be excellent to each other — see
[`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md).
