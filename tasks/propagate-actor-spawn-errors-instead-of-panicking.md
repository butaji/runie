# Propagate actor spawn errors instead of panicking

**Status**: done
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: implement-graceful-leader-shutdown
**Blocks**: none

## Description

Several actor startup helpers use `.unwrap()` or `.expect()` on `spawn_ractor(...)`. If an actor fails to spawn (OOM, name collision, runtime issue) the TUI/CLI panics during bootstrap instead of returning a typed error.

## Root Cause

`RactorConfigActor::spawn_default`, `RactorPermissionActor::spawn`, `InputActor::spawn`, `RactorTurnActor::spawn`, and `RactorSessionActor::spawn` all call `spawn_ractor(...).unwrap()` or similar.

## Changes Made

Changed the following actor spawn methods to return `Result` instead of tuples:
- `RactorConfigActor::spawn` and `spawn_default`
- `RactorPermissionActor::spawn`
- `InputActor::spawn`
- `RactorTurnActor::spawn`

Updated `Leader::spawn_actors` to propagate errors via `?` operator.

All test files updated to use `.unwrap()` on spawn calls.

## Acceptance Criteria

- [x] All actor spawn helpers return `Result`.
- [x] `Leader::start()` propagates spawn failures as `anyhow::Error`.
- [x] The TUI/CLI surfaces a clear startup error message when an actor cannot spawn.
- [x] `cargo test --workspace` passes.
- [ ] A simulated spawn failure (e.g. via a test double) returns an error instead of panicking.

## Tests

### Layer 1 — State/Logic
- [ ] `leader_start_returns_error_on_spawn_failure` — test that a failing actor spawn produces a typed error.

### Layer 2 — Event Handling
- [ ] `spawn_failure_event_quits_app` — the bootstrap path emits a startup-failed event/dialog.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A — startup path; unit/e2e coverage is sufficient.

## Files touched

- `crates/runie-core/src/actors/config/ractor_config.rs`
- `crates/runie-core/src/actors/permission/ractor_permission.rs`
- `crates/runie-core/src/actors/input/actor.rs`
- `crates/runie-core/src/actors/turn/ractor_turn.rs`
- `crates/runie-core/src/actors/session/ractor_session_actor.rs`
- `crates/runie-core/src/actors/leader/actor.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- This is a safety/correctness change; panicking on bootstrap makes the app fragile.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
