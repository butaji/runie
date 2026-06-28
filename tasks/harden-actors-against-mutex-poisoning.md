# Harden async actors against mutex poisoning and startup panics

**Status**: todo
**Milestone**: R5
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: migrate-production-actors-to-ractor
**Blocks**: none

## Description

Actors hold state in `std::sync::Mutex` and call `.lock().unwrap()` everywhere. A panic in an actor task poisons the mutex and causes restart loops in `ractor`. Replace these with `tokio::sync::Mutex` or `parking_lot::Mutex`, and convert binary startup `expect` calls and actor panics into typed errors.

## Acceptance Criteria

- [ ] Replace `std::sync::Mutex` + `.lock().unwrap()` in actor modules with `tokio::sync::Mutex` or `parking_lot::Mutex`.
- [ ] Return `Result`/actor errors from `runie-tui` bootstrap instead of `expect`.
- [ ] Make `runie-agent/src/actor.rs` emit errors instead of panicking on missing handles.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `mutex_state_roundtrip` — state updates work with the new mutex type.

### Layer 2 — Event Handling
- [ ] `actor_handles_missing_handle_gracefully` — a missing handle produces an error event instead of a panic.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `mock_turn_survives_actor_restart` — a provider replay turn completes even if a non-fatal actor error occurs.

## Files touched

- `crates/runie-core/src/actors/**/*.rs`
- `crates/runie-agent/src/actor.rs`
- `crates/runie-tui/src/main.rs`

## Notes

- Prefer `tokio::sync::Mutex` for held-across-await points; `parking_lot::Mutex` is fine for short critical sections.
- This task depends on the actor migration because legacy actor modules may still reference the old mutex patterns.
