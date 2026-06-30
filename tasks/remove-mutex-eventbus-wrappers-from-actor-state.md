# Remove `Mutex<EventBus>` wrappers from actor `State`

**Status**: done
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: use-ractor-state-for-actor-mutable-state
**Blocks**: none

## Description

Several actors held `EventBus<Event>` inside `parking_lot::Mutex` in their ractor `State`. `EventBus` is `Clone` and `publish` takes `&self`, so the `Mutex` was pure overhead. Now all actors hold the bus directly.

## Acceptance Criteria

- [x] Remove `Mutex<EventBus>` from `RactorTurnActor`, `RactorSessionActor`, `InputActor`, `RactorConfigActor`, `RactorPermissionActor` state.
- [x] Update `emit` helpers to take `&self` (no longer needed since publish is already &self).
- [x] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [ ] `actor_emits_event_without_mutex` — state mutation succeeds.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `turn_events_still_reach_ui` — events reach UI after refactor.

## Files touched

- `crates/runie-core/src/actors/turn/ractor_turn.rs`
- `crates/runie-core/src/actors/session/ractor_session_actor.rs`
- `crates/runie-core/src/actors/input/actor.rs`
- `crates/runie-core/src/actors/config/config_handle.rs`
- `crates/runie-core/src/actors/permission/ractor_permission.rs`

## Notes

- `EventBus` is already `Clone`; no locking is needed for publish.
