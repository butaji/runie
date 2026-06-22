# Move SessionStore I/O Off Async Runtime

**Status**: done
**Milestone**: R3
**Category**: Sessions
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

The store's own docs state "Caller should wrap in `spawn_blocking` for async contexts," yet `SessionActor` calls `store.append()` / `store.load_events()` directly inside an async loop. redb write transactions can block the executor for tens to hundreds of milliseconds, stalling the event loop.

## Acceptance Criteria

- [ ] All `SessionStore` I/O inside `SessionActor` runs on `tokio::task::spawn_blocking`.
- [ ] Error handling is preserved across the blocking boundary.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
N/A — mostly runtime behavior.

### Layer 2 — Event Handling
- [ ] `session_actor_appends_durably_without_blocking` — events are appended and acknowledged.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_rapid_events_no_stalls` — high event rate does not hang the TUI.

## Files touched

- `crates/runie-core/src/session_actor.rs`
- `crates/runie-core/src/session_store.rs`

## Notes

Consider wrapping the `SessionStore` methods themselves in a small async facade so callers cannot forget `spawn_blocking`.
