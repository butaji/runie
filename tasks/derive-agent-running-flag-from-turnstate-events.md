# Derive `agent_running` flag from `TurnState` events

## Status

`todo`

## Description

`UiActor` maintains an `agent_running` guard that duplicates the authoritative `TurnState`. Derive it from `TurnState` events and a single in-flight counter owned by `TurnActor`.

## Acceptance criteria

1. **Unit tests** — `agent_running` reflects `TurnStarted`/`TurnComplete` events accurately.
2. **E2E tests** — UI enables/disables submit at the right times in replay.
3. **Live tmux tests** — Submit a prompt, wait for completion, and confirm submit is re-enabled.

## Tests

### Unit tests
- In-flight counter logic.

### E2E tests
- Replay turn verifies submit state changes.

### Live tmux tests
- Submit and complete a turn in tmux.
