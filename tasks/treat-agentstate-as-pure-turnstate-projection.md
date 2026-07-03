# Treat `AgentState` as pure `TurnState` projection

## Status

`todo`

## Description

`AgentState` should be derived entirely from `TurnState` via `From<&TurnState>`. Remove any fields that are updated independently.

## Acceptance criteria

1. **Unit tests** — `AgentState::from(&turn_state)` passes for all representative states.
2. **E2E tests** — Replay produces identical `AgentState` before and after.
3. **Live tmux tests** — Run a turn and verify UI state matches turn state.

## Tests

### Unit tests
- Projection parity for inflight, queues, tokens, speed.

### E2E tests
- Replay comparison.

### Live tmux tests
- Observe status bar and queue display.
