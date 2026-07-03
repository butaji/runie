# Add idempotency keys to turn events

## Status

`todo`

## Description

Every user request, tool call, and turn should carry a stable `request_id` / `turn_id` so replay and duplicate event handling are deterministic.

## Acceptance criteria

1. **Unit tests** — All turn/submission events include stable IDs.
2. **E2E tests** — Replaying the same events twice produces the same state.
3. **Live tmux tests** — Submit identical prompts and verify distinct IDs.

## Tests

### Unit tests
- Events carry IDs; duplicates are rejected/deduplicated.

### E2E tests
- Replay idempotency.

### Live tmux tests
- Submit prompts and check request IDs in logs.
