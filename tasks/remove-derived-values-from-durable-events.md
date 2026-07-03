# Remove derived values from durable events

## Status

`todo`

## Description

`DurableCoreEvent::ToolResult` stores `duration_secs`; other durable events may store computed ratios. Remove derived values and compute them during replay/projection.

## Acceptance criteria

1. **Unit tests** — Durable events contain only raw facts; replay computes derived values.
2. **E2E tests** — Replayed sessions produce the same derived values.
3. **Live tmux tests** — Save and resume a session in tmux.

## Tests

### Unit tests
- Durable event schema has no derived fields.

### E2E tests
- Replay computes duration/ratio.

### Live tmux tests
- Save/resume session.
