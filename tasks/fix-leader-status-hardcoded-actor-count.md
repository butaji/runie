# Fix LeaderHandle status hardcoded actor count

**Status**: done
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P3

**Depends on**: propagate-actor-spawn-errors-instead-of-panicking
**Blocks**: none

## Description

`LeaderHandle::status` reports `actor_count: 9` as a literal. As actors are added or removed, this diagnostic value becomes wrong.

## Root Cause

`crates/runie-core/src/actors/leader/handle.rs:133-139` hardcodes the count.

## Acceptance Criteria

- [x] `actor_count` is computed from the actual spawned handles/cells.
- [x] The status struct stays correct after actor additions/removals.
- [x] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [x] `leader_status_counts_actors` — spawn a leader and assert `status.actor_count` matches the number of spawned actors.

## Files touched

- `crates/runie-core/src/actors/leader/handle.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- Minor correctness issue for diagnostics and logging.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
