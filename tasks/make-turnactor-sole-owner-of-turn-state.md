# Make `TurnActor` the sole owner of turn state

## Status

`todo`

## Description

`TurnActor` should be the single source of truth for `request_queue`, `message_queue`, `inflight`, `turn_active`, streaming state, and token counters. Remove parallel mutation paths.

## Acceptance criteria

1. **Unit tests** — All turn-state mutations go through `TurnActor`; no production code mutates `AppState.turn_state` directly.
2. **E2E tests** — Mock-provider replay turn produces the same final `AppState`.
3. **Live tmux tests** — Run a multi-tool turn in tmux and verify queue/inflight state is correct.

## Tests

### Unit tests
- Static check: no direct `turn_state_mut()` mutation outside `TurnActor`.

### E2E tests
- Replay turn with queued messages.

### Live tmux tests
- Submit multiple messages and observe queue/turn state.
