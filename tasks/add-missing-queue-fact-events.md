# Add missing queue fact events

## Status

`todo`

## Description

Add `QueueFollowUpAdded`, `QueueSteeringAdded`, `QueuesCleared`, and fix `QueueAborted` to remove the message from the queue. Ensure `AppState` projects these.

## Acceptance criteria

1. **Unit tests** — Each queue fact updates `AppState` queue state correctly.
2. **E2E tests** — Queue behavior in replay matches before.
3. **Live tmux tests** — Queue multiple messages and abort/clear them in tmux.

## Tests

### Unit tests
- Queue add/clear/abort projection tests.

### E2E tests
- Replay with queued messages.

### Live tmux tests
- Queue, abort, and clear messages.
