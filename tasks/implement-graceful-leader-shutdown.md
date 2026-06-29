# Implement graceful leader shutdown

**Status**: todo
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: migrate-tui-and-cli-to-leader-bootstrap
**Blocks**: none

## Description

`LeaderHandle::shutdown` publishes `Quit` and exits without stopping child actors. Store child `ActorCell`s and the turn join handle, then stop children and await the join on shutdown.

## Acceptance Criteria

- [ ] `Leader` stores child actor cells and turn join handle.
- [ ] `shutdown` stops all spawned child actors.
- [ ] `shutdown` awaits the turn join handle.
- [ ] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [ ] `leader_shutdown_stops_children` — mock child receives stop after shutdown.

### Layer 2 — Event Handling
- [ ] `shutdown_event_stops_leader` — `Quit` event triggers graceful shutdown.

## Files touched

- `crates/runie-core/src/actors/leader/actor.rs`
- `crates/runie-core/src/actors/leader/mod.rs`

## Notes

- This prevents leaked actor tasks and makes tests deterministic.
