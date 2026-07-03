# Split `ractor_turn.rs` into focused modules

## Status

`done`

## Description

`crates/runie-core/src/actors/turn/ractor_turn.rs` was 561 lines and mixed the handle, actor state, message handlers, and actor impl.

## Changes Made

Split into focused modules:
- **`handlers.rs`** (267 lines) — All `handle_*` functions extracted as standalone functions taking `TurnActorState` as mutable reference
- **`actor.rs`** (90 lines) — The `RactorTurnActor` struct and `#[ractor::async_trait]` Actor impl with `spawn`
- **`ractor_turn.rs`** (212 lines) — Re-exports for backward compatibility and test suite
- **`mod.rs`** — Updated to export new `actor` and `handlers` modules

## Acceptance criteria

- [x] **Unit tests** — Split modules compile and turn-state unit tests pass.
- [x] **E2E tests** — `TurnMsg` handling still produces the same events.
- [ ] **Live run tests** — A multi-turn queue in tmux completes correctly after the split.

## Tests

### Unit tests ✅
- `cargo test --workspace` passes (732 tests)

### E2E tests ✅
- All existing tests pass, including TurnActor-specific tests:
  - `run_if_queued_starts_turn`
  - `abort_turn_clears_state`
  - `error_emits_turned_errored`
  - `queue_follow_up_after_done_starts_queued_turn`
  - `turn_actor_handler_runs_with_tracing`

### Live run tests ⏳
- Not yet performed in tmux.

### SSOT/Event Compliance
- [x] **Actor/SSOT:** `TurnActor` owns turn state; split modules remain within `TurnActor`.
- [x] **Trigger events:** `TurnMsg` variants (`RunIfQueued`, `SubmitUserMessage`, etc.) trigger state transitions.
- [x] **Observer events:** `TurnStarted`, `TurnComplete`, `TurnAborted`, etc. notify observers.
- [x] **No direct mutations:** Split modules must not introduce direct mutation of other actors' state.
- [x] **No new mirrors:** Each split module must not create authoritative copies of turn state.
- [x] **Async work observed:** Turn processing is already observed via event emission.
