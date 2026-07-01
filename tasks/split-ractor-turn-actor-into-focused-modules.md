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
