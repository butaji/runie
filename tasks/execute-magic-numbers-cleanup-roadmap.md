# Execute magic numbers cleanup roadmap

## Status

`todo`

## Description

Track the phased cleanup of magic numbers and hardcoded values. Success metrics: fewer raw literals in production code, single source of truth for constants, tunable values in config.

## Acceptance criteria

1. **Unit tests** — A metrics script counts raw literals and reports reduction.
2. **E2E tests** — Smoke tests pass after each phase.
3. **Live tmux tests** — A full manual tmux session shows no layout/behavior regressions.

## Tests

### Unit tests
- Metrics script counts improvements.

### E2E tests
- Smoke tests after each phase.

### Live tmux tests
- Complete a coding session in tmux.
