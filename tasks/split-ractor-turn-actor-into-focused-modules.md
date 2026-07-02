# Split `ractor_turn.rs` into focused modules

## Status

`todo`

## Description

`crates/runie-core/src/actors/turn/ractor_turn.rs` is 553 lines and mixes the handle, actor state, message handlers, and actor impl.

## Acceptance criteria

1. **Unit tests** — Split modules compile and turn-state unit tests pass.
2. **E2E tests** — `TurnMsg` handling still produces the same events.
3. **Live run tests** — A multi-turn queue in tmux completes correctly after the split.

## Tests

### Unit tests
- Split modules compile and tests pass.

### E2E tests
- `TurnMsg` handling still produces the same events.

### Live run tests
- Queue multiple turns in tmux and verify each completes in order.

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** `TurnActor` owns turn state; split modules remain within `TurnActor`.
- [ ] **Trigger events:** `TurnMsg` variants (`RunIfQueued`, `SubmitUserMessage`, etc.) trigger state transitions.
- [ ] **Observer events:** `TurnStarted`, `TurnComplete`, `TurnAborted`, etc. notify observers.
- [ ] **No direct mutations:** Split modules must not introduce direct mutation of other actors' state.
- [ ] **No new mirrors:** Each split module must not create authoritative copies of turn state.
- [ ] **Async work observed:** Turn processing is already observed via event emission.
