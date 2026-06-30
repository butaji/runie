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

The snapshot receiver was added to the handle but never wired to `UiActor`’s render output.

## Acceptance Criteria

- [ ] Either connect `snapshot_rx` to the actual `UiActor` render channel or remove the field.
- [ ] No dead placeholder code remains in `LeaderHandle`.
- [ ] `cargo test --workspace` passes.
- [ ] Live tmux smoke tests still pass.

## Tests

### Layer 2 — Event Handling
- [ ] `snapshot_rx_receives_render_snapshots` — after a UI update, `snapshot_rx` contains a non-default snapshot.

### Layer 3 — Rendering
- [ ] N/A if field is removed.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A — covered by existing smoke tests.

## Files touched

- `crates/runie-core/src/actors/leader/handle.rs`
- `crates/runie-core/src/actors/leader/actor.rs`
- `crates/runie-tui/src/main.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- If no code uses `snapshot_rx`, removal is the simplest fix.
