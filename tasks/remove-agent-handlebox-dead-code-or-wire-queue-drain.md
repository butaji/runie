# Remove AgentHandleBox dead code or wire run_if_queued

**Status**: todo
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: fix-tui-turn-complete-leaves-working-status-and-queued
**Blocks**: fix-tui-multi-turn-follow-up-stuck-behind-active-turn

## Description

`AgentHandleBox::run` is marked `#[allow(dead_code)]` and `run_if_queued` has no production callers. The queue-draining abstraction exists but is unused, which is confusing and may hide the queue-drain bug.

## Root Cause

The abstraction was added but never wired after the turn queue refactor.

## Acceptance Criteria

- [ ] Either remove the unused methods from `AgentHandleBox` or wire `run_if_queued` so `TurnActor` can drain the queue.
- [ ] The decision is consistent with the queue-drain fix in `TurnActor`.
- [ ] `cargo test --workspace` passes.
- [ ] Live tmux multi-turn scenario works.

## Tests

### Layer 1 — State/Logic
- [ ] `run_if_queued_called_after_done` — if kept, assert `TurnActor::handle_done` invokes it.

### Layer 2 — Event Handling
- [ ] N/A if removed; if kept, covered by queue-drain tests.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_multi_turn_queue_drains` — live tmux script sends two messages and asserts both run.

## Files touched

- `crates/runie-tui/src/ui_actor_agent_handles.rs`
- `crates/runie-core/src/actors/turn/ractor_turn.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- This should be resolved together with the `TurnActor` queue-drain task.
