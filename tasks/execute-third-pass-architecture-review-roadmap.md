# Execute third-pass architecture review roadmap

## Status

`done`
**Supersedes**: create-unified-architecture-backlog-execution-task.md

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

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** N/A (review roadmap; actors remain authoritative).
- [ ] **Trigger events:** N/A (review roadmap doesn't introduce state transitions).
- [ ] **Observer events:** N/A (review roadmap doesn't emit events).
- [ ] **No direct mutations:** N/A (review roadmap doesn't change state ownership).
- [ ] **No new mirrors:** N/A (review roadmap doesn't introduce new state).
- [ ] **Async work observed:** N/A (review roadmap doesn't introduce async work).
