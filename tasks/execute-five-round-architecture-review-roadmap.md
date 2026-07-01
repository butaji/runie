# Execute five-round architecture review roadmap

## Status

`todo`

## Description

Track the integration of the five-round architecture review. Success metrics: fewer LOC, zero production files >500 lines, fewer custom modules, all async work observed.

## Acceptance criteria

1. **Unit tests** — A metrics script counts LOC and files >500 lines and reports improvement.
2. **E2E tests** — Smoke tests pass after each round.
3. **Live run tests** — A full manual session in tmux exercises the changes end-to-end.

## Tests

### Unit tests
- Static metrics script counts LOC and files >500 lines.

### E2E tests
- Smoke tests pass after each round.

### Live run tests
- Run a complete coding session in tmux and verify no regressions.
