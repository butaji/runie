# Execute magic numbers cleanup roadmap

## Status

`done`
**Supersedes**: create-unified-architecture-backlog-execution-task.md

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

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** N/A (cleanup roadmap; actors remain authoritative).
- [ ] **Trigger events:** N/A (cleanup roadmap doesn't introduce state transitions).
- [ ] **Observer events:** N/A (cleanup roadmap doesn't emit events).
- [ ] **No direct mutations:** N/A (cleanup roadmap doesn't change state ownership).
- [ ] **No new mirrors:** N/A (cleanup roadmap doesn't introduce new state).
- [ ] **Async work observed:** N/A (cleanup roadmap doesn't introduce async work).
