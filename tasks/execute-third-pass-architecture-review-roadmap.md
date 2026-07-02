# Execute third-pass architecture review roadmap

## Status

`todo`

## Description

Track integration of the third-pass review. Success metrics: typed errors, deterministic tests, observable actors, faster/feature-gated builds.

## Acceptance criteria

1. **Unit tests** — Metrics script reports fewer stringly errors, fewer real sleeps, fewer unconditional deps.
2. **E2E tests** — Smoke tests pass after each phase.
3. **Live tmux tests** — Full manual tmux session shows no regressions.

## Tests

### Unit tests
- Metrics script counts improvements.

### E2E tests
- Smoke tests after each phase.

### Live tmux tests
- Complete a coding session in tmux.
