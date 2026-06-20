# Move FFF Indexer Initial Scan Off Async Runtime

**Status**: done
**Milestone**: R3
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

`FffIndexerActor` calls `shared_picker.wait_for_scan(timeout)`, a blocking LMDB/filesystem operation, from within an async function without `spawn_blocking`. This can stall other actors during startup.

## Acceptance Criteria

- [ ] Picker initialization / scan wait runs inside `tokio::task::spawn_blocking` or uses an async-friendly wait mechanism.
- [ ] `FffIndexerActor` does not block the executor on startup.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
N/A.

### Layer 2 — Event Handling
- [ ] `fff_indexer_ready_event_after_scan` — ready event is emitted after scan completes.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_startup_no_stall` — startup does not hang before the TUI appears.

## Files touched

- `crates/runie-core/src/actors/fff_indexer/search.rs`
- `crates/runie-core/src/actors/fff_indexer/mod.rs`

## Notes

Coordinate with `session-store-blocking-io` to keep blocking patterns consistent across actors.
