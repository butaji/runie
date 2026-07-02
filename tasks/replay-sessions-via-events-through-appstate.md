# Replay sessions via events through `AppState`

## Status

`todo`

## Description

`session/replay.rs` directly mutates `session_mut()` instead of applying events. Replay should emit `Event`s and update `AppState` through `AppState::update`.

## Acceptance criteria

1. **Unit tests** — Replaying a saved session produces the same `AppState` as direct mutation.
2. **E2E tests** — Session replay works in mock-provider tests.
3. **Live tmux tests** — Resume a session in tmux and verify the chat tree is restored.

## Tests

### Unit tests
- Event-by-event replay matches old state.

### E2E tests
- Replay fixture loads a session.

### Live tmux tests
- Save and resume a session in tmux.
