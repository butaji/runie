# Replace whole-state `SessionChanged` with fine-grained events

## Status

`todo`

## Description

`SessionActor` emits `Event::SessionChanged { state }` carrying the whole `SessionState`. Replace with fine-grained events so subscribers rebuild from facts, not from the boxed state.

## Acceptance criteria

1. **Unit tests** — Fine-grained events cover all `SessionState` changes.
2. **E2E tests** — `AppState` projection from fine-grained events matches old behavior.
3. **Live tmux tests** — Edit session metadata in tmux and observe updates.

## Tests

### Unit tests
- Each fine-grained event updates the projection.

### E2E tests
- Replay with session changes.

### Live tmux tests
- Rename or delete a session.
