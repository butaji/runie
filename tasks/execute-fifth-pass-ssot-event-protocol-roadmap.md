# Execute fifth-pass SSOT event protocol roadmap

## Status

`todo`

## Description

Track the implementation of the fifth-pass findings. Success metrics: `TurnActor` is the sole turn authority, events are idempotent, derived values removed from events, no unsafe reply ports.

## Acceptance criteria

1. **Unit tests** — Static and unit tests verify SSOT, idempotency, and no unsafe ports.
2. **E2E tests** — All replay fixtures pass.
3. **Live tmux tests** — Full manual tmux session completes without state drift.

## Tests

### Unit tests
- SSOT and idempotency guards.

### E2E tests
- Full replay suite.

### Live tmux tests
- Complete a multi-tool session in tmux.
