# Execute second-pass architecture review roadmap

## Status

`todo`

## Description

Track integration of the second-pass review using the Pareto prioritization in `docs/superpowers/plans/2026-06-28-second-pass-pareto-prioritization.md`. Success metrics: fewer LOC, fewer hand-maintained tables, centralized HTTP/retry, event-driven TUI, JSONL persistence.

## Acceptance criteria

1. **Unit tests** — A metrics script reports reductions in LOC, `Event` match-table lines, and duplicate HTTP client builders.
2. **E2E tests** — Smoke tests pass after each round.
3. **Live tmux tests** — A full manual tmux session validates no regressions.

## Tests

### Unit tests
- Metrics script counts improvements.

### E2E tests
- Smoke tests after each round.

### Live tmux tests
- Run a complete coding session in tmux.
