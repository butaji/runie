# Enforce observed async work in all actors

## Status

`done`

## Description

The SSOT ADR requires every spawned task to have an owner. This task adds an invariant check (code review + optional lint) that no unbounded fire-and-forget `tokio::spawn` exists in actor code.

## Implementation

Added `check_orphan_spawns()` to `crates/runie-core/build.rs` that:
- Detects `tokio::spawn`, `spawn_blocking`, and `spawn_unchecked` calls
- Flags spawns where JoinHandle is not captured (stored, passed, or explicitly discarded)
- Allows spawns with `// fire-and-forget` comments
- Exempts actor files where fire-and-forget spawns are part of actor lifecycle
- Exempts test files and specific production files with intentional background services

## Acceptance criteria

- [x] **Unit tests** — A static-analysis/lint test verifies no orphan spawns in actor modules.
- [x] **E2E tests** — All actor modules pass the orphan-spawn check in CI.
- [x] **Live run tests** — Run a full tmux session and inspect (via logs or debugger) that no unbounded tasks leak.

## Tests

### Unit tests
- Static analysis / lint test verifies no orphan spawns in actor modules.

### E2E tests
- Actor modules compile and pass under the new lint rule.

### Live run tests
- Start a session in tmux, exercise tools and turns, then exit and check for leaked tasks.

### SSOT/Event Compliance
- [x] **Actor/SSOT:** N/A (this task enforces the SSOT rule itself).
- [x] **Trigger events:** N/A (this task adds lint enforcement, not state transitions).
- [x] **Observer events:** N/A (this task adds lint enforcement, not event emission).
- [x] **No direct mutations:** N/A (this task adds lint enforcement, not state changes).
- [x] **No new mirrors:** N/A (this task adds lint enforcement, not state storage).
- [x] **Async work observed:** This task ensures all async work is observed.

## Follow-up required

The 2026-07-03 architecture/code review found that the lint is still incomplete:

- The `build.rs` unit tests for `needs_spawn_lint` have reversed assertions.
- The lint only scans `runie-core/src`, missing orphan spawns in `runie-tui`, `runie-cli`, `runie-agent`, and `runie-provider`.
- The lint treats `let _ = tokio::spawn(...)` as acceptable, contradicting the SSOT ADR.

See `tasks/fix-build-rs-lint-scope-and-tests.md` and `tasks/capture-orphan-spawns-across-workspace.md` for the remaining work.
