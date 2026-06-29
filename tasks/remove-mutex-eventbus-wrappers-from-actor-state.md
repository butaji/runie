# Remove `Mutex<EventBus>` wrappers from actor `State`

**Status**: todo
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: use-ractor-state-for-actor-mutable-state
**Blocks**: none

## Description

Several actors hold `EventBus<Event>` inside `parking_lot::Mutex` in their ractor `State`. `EventBus` is `Clone` and `publish` takes `&self`, so the `Mutex` is pure overhead. Hold the bus directly and change `emit` helpers to `&mut self`.

## Acceptance Criteria

- [ ] Remove `Mutex<EventBus>` from `RactorTurnActor`, `RactorSessionActor`, `InputActor`, `RactorConfigActor`, `RactorPermissionActor` state.
- [ ] Update `emit` helpers to take `&mut self`.
- [ ] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [ ] `actor_emits_event_without_mutex` — state mutation succeeds.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `turn_events_still_reach_ui` — events reach UI after refactor.

## Files touched

- `crates/runie-core/src/actors/turn/ractor_turn.rs`
- `crates/runie-core/src/actors/session/ractor_session_actor.rs`
- `crates/runie-core/src/actors/input/actor.rs`
- `crates/runie-core/src/actors/config/ractor_config.rs`
- `crates/runie-core/src/actors/permission/ractor_permission.rs`

## Notes

- `EventBus` is already `Clone`; no locking is needed for publish.
