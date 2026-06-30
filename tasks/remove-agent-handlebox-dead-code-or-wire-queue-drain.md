# Remove AgentHandleBox dead code or wire run_if_queued

**Status**: done
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: fix-tui-turn-complete-leaves-working-status-and-queued
**Blocks**: fix-tui-multi-turn-follow-up-stuck-behind-active-turn

## Description

`AgentHandleBox::run` is marked `#[allow(dead_code)]` and `run_if_queued` has no production callers. The queue-draining abstraction exists but is unused, which is confusing and may hide the queue-drain bug.

## Root Cause

The abstraction was added but never wired after the turn queue refactor.

## Fix Applied

Wired `run_if_queued` in `UiActor::handle_event_inner` after `Done`/`TurnErrored`/`Abort` events:

```rust
if matches!(&evt, Event::Done { .. } | Event::TurnErrored { .. } | Event::Abort) {
    self.agent_running = false;
    if let Some(ref turn_handle) = self.turn_handle {
        self.agent_handle.run_if_queued(turn_handle).await;
    }
}
```

This ensures queued follow-up messages are drained after the current turn completes.

## Acceptance Criteria

- [x] Either remove the unused methods from `AgentHandleBox` or wire `run_if_queued` so `TurnActor` can drain the queue.
- [x] The decision is consistent with the queue-drain fix in `TurnActor`.
- [x] `cargo test --workspace` passes.
- [ ] Live tmux multi-turn scenario works.

## Tests

### Layer 1 — State/Logic
- [ ] `run_if_queued_called_after_done` — if kept, assert `TurnActor::handle_done` invokes it.

### Layer 2 — Event Handling
- [x] N/A if removed; if kept, covered by queue-drain tests.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_multi_turn_queue_drains` — live tmux script sends two messages and asserts both run.

## Files touched

- `crates/runie-tui/src/ui_actor.rs` (wired `run_if_queued` after Done events)
- `crates/runie-tui/src/ui_actor_agent_handles.rs` (kept - methods are used)
- `crates/runie-core/src/actors/turn/ractor_turn.rs` (unchanged - already has `RunIfQueued` handler)

## Validation

- `cargo check --workspace`: passes with no warnings
- `cargo test --workspace`: 733 passed, 0 failed
