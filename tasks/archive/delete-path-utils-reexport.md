# Delete runie-agent path_utils re-export

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P3

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-agent/src/path_utils.rs` is a 3-line pure re-export shim: `pub use runie_core::path::resolve_path;`. Declared as `pub mod path_utils;` in `lib.rs:7` but no file imports `runie_agent::path_utils` or `crate::path_utils` (grep returned only the `pub mod` line itself). Dead public API surface suggesting the agent has its own path logic.

## Acceptance Criteria

- [ ] `crates/runie-agent/src/path_utils.rs` deleted.
- [ ] `pub mod path_utils;` removed from `crates/runie-agent/src/lib.rs`.
- [ ] `rg "path_utils" crates/` returns zero hits.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- N/A — deletion.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_agent_path_resolution_unchanged` — path resolution still works via `runie_core::path`.

## Files touched

- `crates/runie-agent/src/path_utils.rs`
- `crates/runie-agent/src/lib.rs`

## Notes

Trivial. Callers (if any appear) should import `runie_core::path::resolve_path` directly.
