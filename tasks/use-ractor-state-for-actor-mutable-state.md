# Use `ractor` `State` for actor mutable state

**Status**: done
**Milestone**: R6
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: migrate-production-actors-to-ractor
**Blocks**: fix-ractor-permission-actor-reply-lifecycle

## Description

Production actors use `type State = ()` and hold mutable state behind `parking_lot::Mutex`, `Arc<Mutex<...>>`, or `Mutex<Option<...>>`. `ractor` serializes messages per actor, so state can live in `type State = ...` and be mutated via `&mut State` in `handle`. This removes a lot of boilerplate and matches idiomatic `ractor` usage.

## Acceptance Criteria

- [x] Convert `RactorSessionActor`, `RactorTurnActor`, `RactorConfigActor`, `RactorPermissionActor`, and `InputActor` to use `type State` for mutable state.
- [x] Remove interior `Mutex`/`Arc<Mutex>` fields where they are only used for actor-local state.
- [x] Keep `Mutex` only for state shared across threads outside the actor (e.g., `ApprovalRegistry` which is process-wide).
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `session_actor_state_updates_without_mutex` — state updates via `&mut State`.
- [x] `turn_actor_state_updates_without_mutex` — same.

### Layer 2 — Event Handling
- [x] `config_actor_reload_updates_state` — reload message updates `State`.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `mock_turn_still_completes` — provider replay turn works after state refactor.

## Files touched

- `crates/runie-core/src/actors/session/ractor_session_actor.rs`
- `crates/runie-core/src/actors/turn/ractor_turn.rs`
- `crates/runie-core/src/actors/config/ractor_config.rs`
- `crates/runie-core/src/actors/permission/ractor_permission.rs`
- `crates/runie-core/src/actors/input/actor.rs` (already done)

## Notes

- `ctx7` for `ractor` confirms the pattern: `type State = ...`, `handle(..., state: &mut State)`.
- This should land before permission-actor reply lifecycle because the fix becomes trivial with state as a `HashMap`.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
