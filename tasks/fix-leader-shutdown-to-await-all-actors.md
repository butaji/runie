# Fix leader shutdown to await all actors

**Status**: todo
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: implement-graceful-leader-shutdown
**Blocks**: none

## Description

`LeaderHandle::shutdown` uses `Arc::try_unwrap(...).expect(...)` which panics if any clone of the handle/agent join handle still exists. It also only awaits the turn and agent joins; other actor cells are stopped without waiting for clean termination.

## Root Cause

Shutdown ownership is not tracked carefully. The handle expects to be the sole owner at shutdown time, which is brittle.

## Acceptance Criteria

- [ ] Graceful shutdown never panics, even if handles are cloned elsewhere.
- [ ] All actor join handles are awaited before the leader returns.
- [ ] A shutdown signal/channel is used so actors can finish in-flight work.
- [ ] `cargo test --workspace` passes.
- [ ] A simulated shutdown while a turn is active terminates cleanly.

## Tests

### Layer 1 — State/Logic
- [ ] `leader_shutdown_awaits_all_actors` — shutdown joins every actor handle.
- [ ] `leader_shutdown_does_not_panic_with_cloned_handle` — clone the handle, then shutdown.

### Layer 2 — Event Handling
- [ ] `shutdown_signal_stops_turn_actor` — a shutdown signal terminates an active turn.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_quit_during_turn_exits_cleanly` — live tmux script starts `hello`, presses quit, and asserts no panic output.

## Files touched

- `crates/runie-core/src/actors/leader/handle.rs`
- `crates/runie-core/src/actors/leader/actor.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- This is part of graceful shutdown; needed for clean TUI exit and CLI server mode.
