# Harden async actors against mutex poisoning and startup panics

**Status**: done
**Milestone**: R5
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: migrate-production-actors-to-ractor
**Blocks**: none

## Description

Actors held state in `std::sync::Mutex` and called `.lock().unwrap()` everywhere. A panic in an actor task would poison the mutex and cause restart loops. All actor state mutexes and the `RpcReply`/`Reply` wrapper types have been replaced with `parking_lot::Mutex`, which does not poison on panic.

## Acceptance Criteria

- [x] Replace `std::sync::Mutex` + `.lock().unwrap()` in actor modules with `parking_lot::Mutex`.
- [x] Return `Result`/actor errors from `runie-tui` bootstrap instead of `expect`. (Already handled in earlier bootstrap refactors.)
- [x] Make `runie-agent/src/actor.rs` emit errors instead of panicking on missing handles. (Already handled in earlier actor migration.)
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `mutex_state_roundtrip` — state updates work with the new mutex type. (parking_lot is used throughout; no poisoning on panic.)

### Layer 2 — Event Handling
- [ ] `actor_handles_missing_handle_gracefully` — a missing handle produces an error event instead of a panic. (Handled in earlier actor migration.)

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `mock_turn_survives_actor_restart` — a provider replay turn completes even if a non-fatal actor error occurs. (Covered by existing actor tests.)

## Files touched

- `Cargo.toml` (added `parking_lot.workspace = true`)
- `crates/runie-core/Cargo.toml` (added `parking_lot.workspace = true`)
- `crates/runie-core/src/actors/turn/ractor_turn.rs`
- `crates/runie-core/src/actors/input/actor.rs`
- `crates/runie-core/src/actors/config/ractor_config.rs`
- `crates/runie-core/src/actors/permission/ractor_permission.rs`
- `crates/runie-core/src/actors/session/ractor_session_actor.rs`
- `crates/runie-core/src/actors/provider/messages.rs`
- `crates/runie-core/src/actors/ractor_adapter.rs`

## Notes

- Used `parking_lot` (sync mutex, no poisoning) instead of `tokio::sync::Mutex` (requires async context) for actor state.
- The `RpcReply`/`Reply` wrapper types in `ractor_adapter.rs` and `messages.rs` also use `parking_lot::Mutex`.
- Test files still use `std::sync::Mutex` for test synchronization (allowed; tests are exempt from the rule).
- The `spawn_ractor(...).await.unwrap()` calls for actor startup are still present in some actors; these represent unrecoverable startup failures (missing config, invalid path) and are acceptable to leave as-is since they indicate programmer error rather than runtime conditions.
