# Remove unused cargo dependencies

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

10 truly unused deps + 3 redundant dev-deps + 1 unused workspace dep. Verified by grep for `use <crate>` / `<crate>::` across each crate's `src/`, `tests/`, `build.rs`.

## Acceptance Criteria

- [ ] runie-tui: `pulldown-cmark` removed (delegates to `runie_core::markdown`; 0 refs).
- [ ] runie-core: `reqwest` removed (only a comment in `retry.rs:54`).
- [ ] runie-core: `strum` removed (0 refs, no derives).
- [ ] runie-provider: `dirs` removed (0 refs).
- [ ] runie-server: `serde` and `futures` removed (0 refs).
- [ ] runie-print: `futures` removed (0 refs).
- [ ] runie-testing: `serde_json` removed (0 refs).
- [ ] runie-tui dev-deps: `regex`, `libc`, `rexpect` removed (no `tests/` dir; `rexpect` violates AGENTS.md no-tmux rule).
- [ ] runie-agent dev-dep: `insta` removed (0 snapshot usage).
- [ ] Redundant dev-deps deduped: runie-core `parking_lot`, runie-protocol `serde_json`, runie-testing `tokio` (already normal deps).
- [ ] Workspace root: `rexpect` removed from `[workspace.dependencies]`.
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- N/A — dep manifest cleanup.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_workspace_builds_after_dep_prune` — `cargo build --workspace` green.
- [ ] `smoke_lockfile_compact` — `Cargo.lock` does not regress to include removed transitive crates.

## Files touched

- `Cargo.toml` (root)
- `crates/runie-core/Cargo.toml`
- `crates/runie-tui/Cargo.toml`
- `crates/runie-provider/Cargo.toml`
- `crates/runie-server/Cargo.toml`
- `crates/runie-print/Cargo.toml`
- `crates/runie-testing/Cargo.toml`
- `crates/runie-agent/Cargo.toml`
- `crates/runie-protocol/Cargo.toml`
- `Cargo.lock` (regenerated)

## Notes

Investigate (do not auto-remove) underused: runie-core `chrono` (1 file), `textwrap` (1 call), runie-engine `git2` (only `git2::Status` enum), runie-tui `unicode_width` (could reuse `runie_core::display_width`). These are justified-by-feature; flag only.
