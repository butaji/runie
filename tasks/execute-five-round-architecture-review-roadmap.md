# Execute five-round architecture review roadmap

## Status

`done`
**Supersedes**: create-unified-architecture-backlog-execution-task.md

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

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** N/A (review roadmap; actors remain authoritative).
- [ ] **Trigger events:** N/A (review roadmap doesn't introduce state transitions).
- [ ] **Observer events:** N/A (review roadmap doesn't emit events).
- [ ] **No direct mutations:** N/A (review roadmap doesn't change state ownership).
- [ ] **No new mirrors:** N/A (review roadmap doesn't introduce new state).
- [ ] **Async work observed:** N/A (review roadmap doesn't introduce async work).
