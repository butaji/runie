# Make projection handlers idempotent

## Status

`todo`

## Description

Guard `apply_turn_started`, `start_tool`, `append_response`, `finish_turn`, `add_error`, `apply_queue_aborted`, and `TokenStatsUpdated` so duplicate events do not mutate state twice.

## Acceptance criteria

1. **Unit tests** — Applying the same event twice leaves state unchanged after the first application.
2. **E2E tests** — Replay with duplicate events is safe.
3. **Live tmux tests** — Not applicable; logic task.

## Tests

### Unit tests
- Double-application tests for each handler.

### E2E tests
- Replay with intentionally duplicated events.

### Live tmux tests
- N/A.
