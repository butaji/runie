# Execute second-pass architecture review roadmap

## Status

`done`
**Supersedes**: create-unified-architecture-backlog-execution-task.md

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

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** N/A (review roadmap; actors remain authoritative).
- [ ] **Trigger events:** N/A (review roadmap doesn't introduce state transitions).
- [ ] **Observer events:** N/A (review roadmap doesn't emit events).
- [ ] **No direct mutations:** N/A (review roadmap doesn't change state ownership).
- [ ] **No new mirrors:** N/A (review roadmap doesn't introduce new state).
- [ ] **Async work observed:** N/A (review roadmap doesn't introduce async work).
