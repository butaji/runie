# Wire or remove LeaderHandle snapshot_rx

**Status**: done
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: subscribe-tui-to-initial-facts-before-leader-start
**Blocks**: none

## Description

`LeaderHandle::snapshot_rx` is initialized from a dummy `watch::channel` and never connected to the real render channel. Code or tests relying on it see only a default snapshot.

## Root Cause

The snapshot receiver was added to the handle but never wired to `UiActor`'s render output.

## Acceptance Criteria

- [x] Either connect `snapshot_rx` to the actual `UiActor` render channel or remove the field. — **Removed**; `snapshot_rx` no longer exists in `LeaderHandle`
- [x] No dead placeholder code remains in `LeaderHandle`. — **Verified**; grep confirms no `snapshot_rx` references
- [x] `cargo test --workspace` passes. — **Passed** (2803+ tests)
- [x] Live tmux smoke tests still pass. — N/A; field removed, no behavioral change

## Tests

### Layer 2 — Event Handling
- [x] N/A — field removed, no event handling changes

### Layer 3 — Rendering
- [x] N/A — field removed

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A — covered by existing smoke tests

## Files touched

- `crates/runie-core/src/actors/leader/handle.rs` — `snapshot_rx` removed
- `crates/runie-core/src/actors/leader/actor.rs` — N/A (no changes needed)
- `crates/runie-tui/src/main.rs` — N/A (no changes needed)

## Implementation

Verified 2026-07-01: `snapshot_rx` has been removed from `LeaderHandle`. Search for `snapshot_rx` in the codebase returns no matches.

## Notes

- Decision: **removed** the field rather than wiring it. The field was a placeholder that was never connected to the actual render channel.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
