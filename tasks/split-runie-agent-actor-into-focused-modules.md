# Split `runie-agent/src/actor.rs` into focused modules

## Status

`todo`

## Description

`crates/runie-agent/src/actor.rs` is 578 lines and mixes messages, actor impl, turn setup, factory, and leader integration.

## Acceptance criteria

1. **Unit tests** — Split modules compile and agent unit tests pass.
2. **E2E tests** — `AgentMsg` handling still works in a replay turn.
3. **Live run tests** — Run an agent turn in tmux and verify the same lifecycle events.

## Tests

### Unit tests
- Split modules compile and tests pass.

### E2E tests
- `AgentMsg` handling still works.

### Live run tests
- Submit a prompt in tmux and confirm the turn completes with `TurnComplete`.
