# Remove dual agent event application in dispatch

## Status

`todo`

## Description

`update/dispatch.rs` applies agent events to `AppState` and also forwards them to `TurnActor`, which re-emits them. Remove the local application; `AppState` should project only from `TurnActor` events.

## Acceptance criteria

1. **Unit tests** — `dispatch_event` no longer calls `agent_event` for turn facts; `AppState` updates only from `TurnActor`-emitted events.
2. **E2E tests** — Replay turns no longer double-apply facts; no infinite `Thinking` loop.
3. **Live tmux tests** — Submit a prompt and verify the streaming response is not duplicated.

## Tests

### Unit tests
- `AppState` update count matches `TurnActor` event count.

### E2E tests
- Replay fixture with `Thinking` events.

### Live tmux tests
- Submit a prompt and watch for duplicated thinking text.
