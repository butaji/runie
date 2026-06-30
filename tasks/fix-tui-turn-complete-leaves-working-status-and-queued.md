# Fix TUI turn-complete leaves Working status and queued request

**Status**: partial
**Milestone**: R7
**Category**: TUI / Rendering
**Priority**: P0

**Depends on**: fix-tui-mock-simple-text-response-repetition
**Blocks**: fix-tui-multi-turn-follow-up-stuck-behind-active-turn

## Description

After a tool turn completes (e.g. `list files` in mock mode), the assistant area correctly shows `Turn completed in 0.0s`, but the status bar still reads `Working... 0.0s (1 queued)` and the input hint still shows steering/follow-up keys. The queued request is not cleared, so the TUI never returns to the idle state.

## Root Cause

`apply_turn_started` did not pop the message being processed from `request_queue`. When a turn started, the queue count stayed at 1 even though the message was being processed.

## Fix Applied

Added `request_queue.pop_front()` to `apply_turn_started()` so the queue count reflects only messages still waiting:

```rust
pub(crate) fn apply_turn_started(&mut self) {
    self.agent_state_mut().turn_active = true;
    self.agent_state_mut().inflight += 1;
    self.agent_state_mut().streaming = true;
    self.agent_state_mut().turn_started_at = Some(std::time::Instant::now());
    // Pop the message that is being processed from the queue.
    // This ensures queue_count in the snapshot reflects only waiting messages.
    self.agent_state_mut().request_queue.pop_front();
}
```

Also wired `run_if_queued` after `Done` events to drain queued follow-ups.

## Acceptance Criteria

- [x] After a turn reaches `TurnComplete`/`Done`, the status bar leaves `Working...` and shows the idle prompt.
- [x] The queued-request counter drops to zero.
- [ ] The input hint returns to the idle set (no `enter steer` / `alt+enter follow-up`).
- [x] `cargo test --workspace` passes.
- [ ] Live tmux `list files` scenario shows an idle status after `Turn completed`.

## Tests

### Layer 1 — State/Logic
- [ ] `turn_complete_clears_queue_and_status` — after a completed turn, assert `turn_active == false` and the request queue is empty.

### Layer 2 — Event Handling
- [ ] `done_event_updates_status_to_idle` — feed `Event::Done`/`Event::TurnComplete` and assert the idle state events are emitted.

### Layer 3 — Rendering
- [ ] `completed_turn_renders_idle_status` — `TestBackend` asserts the status line no longer contains `Working` after completion.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_list_files_reaches_idle` — live tmux script checks the captured pane for `Turn completed` followed by an idle status.

## Files touched

- `crates/runie-core/src/model/state/turn_projections.rs` (added queue pop in `apply_turn_started`)
- `crates/runie-tui/src/ui_actor.rs` (wired `run_if_queued` after Done events)

## Validation

- `cargo check --workspace`: passes
- `cargo test --workspace`: 733 passed, 0 failed
