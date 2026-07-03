# Add contract tests for `TurnActor` and `SessionStore`

## Status

`todo`

## Description

Define a contract test suite for `TurnActor` message handling and `SessionStore` persistence: idempotency, ordering, crash recovery, and duplicate rejection.

## Acceptance criteria

1. **Unit tests** — Contract tests pass for `TurnActor` and `SessionStore`.
2. **E2E tests** — Contract tests exercise replay and actor interaction.
3. **Live tmux tests** — Not applicable.

## Tests

### Unit tests
- Idempotency, ordering, crash recovery, duplicate rejection.

### E2E tests
- End-to-end contract test with mock provider.

### Live tmux tests
- N/A.
