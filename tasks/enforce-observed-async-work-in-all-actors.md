# Enforce observed async work in all actors

## Status

`todo`

## Description

The SSOT ADR requires every spawned task to have an owner. This task adds an invariant check (code review + optional lint) that no unbounded fire-and-forget `tokio::spawn` exists in actor code.

## Acceptance criteria

1. **Unit tests** — A static-analysis/lint test verifies no orphan spawns in actor modules.
2. **E2E tests** — All actor modules pass the orphan-spawn check in CI.
3. **Live run tests** — Run a full tmux session and inspect (via logs or debugger) that no unbounded tasks leak.

## Tests

### Unit tests
- Static analysis / lint test verifies no orphan spawns in actor modules.

### E2E tests
- Actor modules compile and pass under the new lint rule.

### Live run tests
- Start a session in tmux, exercise tools and turns, then exit and check for leaked tasks.
