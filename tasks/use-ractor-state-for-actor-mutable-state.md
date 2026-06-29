# Use `ractor` `State` for actor mutable state

**Status**: todo
**Milestone**: R6
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: migrate-production-actors-to-ractor
**Blocks**: fix-ractor-permission-actor-reply-lifecycle

## Description

Production actors use `type State = ()` and hold mutable state behind `parking_lot::Mutex`, `Arc<Mutex<...>>`, or `Mutex<Option<...>>`. `ractor` serializes messages per actor, so state can live in `type State = ...` and be mutated via `&mut State` in `handle`. This removes a lot of boilerplate and matches idiomatic `ractor` usage.

## Acceptance Criteria

- [ ] Convert `RactorSessionActor`, `RactorTurnActor`, `RactorConfigActor`, `RactorPermissionActor`, and `InputActor` to use `type State` for mutable state.
- [ ] Remove interior `Mutex`/`Arc<Mutex>` fields where they are only used for actor-local state.
- [ ] Keep `Mutex` only for state shared across threads outside the actor.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `session_actor_state_updates_without_mutex` — state updates via `&mut State`.
- [ ] `turn_actor_state_updates_without_mutex` — same.

### Layer 2 — Event Handling
- [ ] `config_actor_reload_updates_state` — reload message updates `State`.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `mock_turn_still_completes` — provider replay turn works after state refactor.

## Files touched

- `crates/runie-core/src/actors/session/ractor_session_actor.rs`
- `crates/runie-core/src/actors/turn/ractor_turn.rs`
- `crates/runie-core/src/actors/config/ractor_config.rs`
- `crates/runie-core/src/actors/permission/ractor_permission.rs`
- `crates/runie-core/src/actors/input/actor.rs`
- `crates/runie-agent/src/actor.rs`

## Notes

- `ctx7` for `ractor` confirms the pattern: `type State = ...`, `handle(..., state: &mut State)`.
- This should land before permission-actor reply lifecycle because the fix becomes trivial with state as a `HashMap`.
