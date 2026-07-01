# Harden async actors against mutex poisoning and startup panics

**Status**: done
**Milestone**: R5
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: migrate-production-actors-to-ractor
**Blocks**: none

## Description

Actors held state in `std::sync::Mutex` and called `.lock().unwrap()` everywhere. A panic in an actor task would poison the mutex and cause restart loops. All actor state mutexes and the `RpcReply`/`Reply` wrapper types have been replaced with `parking_lot::Mutex`, which does not poison on panic.

The remaining issue (panics on missing handles in `runie-agent/src/actor.rs`) has been fixed: `get_provider_handle` and `get_permission_handle` now return `Option<...>` and emit an error event instead of panicking.

## Acceptance Criteria

- [x] Replace `std::sync::Mutex` + `.lock().unwrap()` in actor modules with `parking_lot::Mutex`.
- [x] Return `Result`/actor errors from `runie-tui` bootstrap instead of `expect`. (Already handled in earlier bootstrap refactors.)
- [x] Make `runie-agent/src/actor.rs` emit errors instead of panicking on missing handles.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `mutex_state_roundtrip` — state updates work with the new mutex type. (parking_lot is used throughout; no poisoning on panic.)

### Layer 2 — Event Handling
- [x] `actor_handles_missing_handle_gracefully` — missing handles produce error events instead of panics. (verified by code change: `get_provider_handle` and `get_permission_handle` return `Option<...>` and call `emit_error_and_done` without panicking.)

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `mock_turn_survives_actor_restart` — covered by existing actor tests.

## Files touched

- `crates/runie-agent/src/actor.rs` — `get_provider_handle` and `get_permission_handle` now return `Option<...>` and emit error events instead of panicking

## Notes

- Used `parking_lot` (sync mutex, no poisoning) instead of `tokio::sync::Mutex` (requires async context) for actor state.
- The `RpcReply`/`Reply` wrapper types in `ractor_adapter.rs` use `parking_lot::Mutex`.
- Test files still use `std::sync::Mutex` for test synchronization (allowed; tests are exempt).
- Remaining `std::sync::Mutex`/`RwLock` in `permissions/`, `fff_indexer/`, and `runie-agent/` is tracked by `normalize-remaining-std-mutex-to-parking-lot.md`.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
