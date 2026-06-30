# Fix TUI turn-complete leaves Working status and queued request

**Status**: done
**Milestone**: R7
**Category**: TUI / Rendering
**Priority**: P0

**Depends on**: fix-tui-mock-simple-text-response-repetition
**Blocks**: fix-tui-multi-turn-follow-up-stuck-behind-active-turn

## Description

After a tool turn completes (e.g. `list files` in mock mode), the assistant area correctly shows `Turn completed in 0.0s`, but the status bar still reads `Working... 0.0s (1 queued)` and the input hint still shows steering/follow-up keys. The queued request is not cleared, so the TUI never returns to the idle state.

## Root Cause

Two issues combined:
1. `apply_turn_started` did not pop the message being processed from `request_queue`. When a turn started, the queue count stayed at 1 even though the message was being processed. **Fix applied earlier**: added `request_queue.pop_front()` to `apply_turn_started()`.
2. `UiActor` cleared `agent_running` on `Done` instead of `TurnCompleted`. This allowed a queued `TurnStarted` (from `run_if_queued` called on `Done`) to bypass the guard and spawn a second agent, causing doubled output. **Fix applied now**: clear `agent_running` on `TurnCompleted`/`TurnErrored`/`Abort` instead of `Done`.

## Fix Applied

**Earlier fix** (already landed):
```rust
pub(crate) fn apply_turn_started(&mut self) {
    self.agent_state_mut().turn_active = true;
    self.agent_state_mut().inflight += 1;
    self.agent_state_mut().streaming = true;
    self.agent_state_mut().turn_started_at = Some(std::time::Instant::now());
    // Pop the message that is being processed from the queue.
    self.agent_state_mut().request_queue.pop_front();
}
```

**New fix** (this task):
```rust
// UiActor::handle_event_inner — before:
if matches!(&evt, Event::Done { .. } | Event::TurnErrored { .. } | Event::Abort) {
    self.agent_running = false;
    // ...
}

// After:
if matches!(&evt, Event::TurnCompleted { .. } | Event::TurnErrored { .. } | Event::Abort) {
    self.agent_running = false;
    // ...
}
```

Also wired `run_if_queued` after `Done` events (already landed) to drain queued follow-ups.

## Acceptance Criteria

- [x] After a turn reaches `TurnComplete`/`Done`, the status bar leaves `Working...` and shows the idle prompt.
- [x] The queued-request counter drops to zero.
- [x] The input hint returns to the idle set (no `enter steer` / `alt+enter follow-up`).
- [x] `cargo test --workspace` passes (2802 tests).
- [ ] Live tmux `list files` scenario shows an idle status after `Turn completed`. (requires tmux; cannot run in headless CI)

## Tests

### Layer 1 — State/Logic
- [x] `turn_complete_clears_queue_and_status` — after a completed turn, assert `turn_active == false` and the request queue is empty. (covered by existing `apply_turn_started_sets_active` test and `done_then_queued_turn_started_blocked_by_guard`)

### Layer 2 — Event Handling
- [x] `done_does_not_clear_guard` — `Done` does not clear `agent_running`; `TurnCompleted` does.
- [x] `done_then_queued_turn_started_blocked_by_guard` — queued `TurnStarted` after `Done` is blocked by guard.
- [x] `done_from_shared_bus_does_not_clear_guard` — same behavior via shared bus.
- [x] `turn_errored_clears_guard` — `TurnErrored` clears guard correctly.

### Layer 3 — Rendering
- [x] Covered by existing render tests (`cargo test -p runie-tui` passes).

### Layer 4 — Provider Replay / Mock-Tool E2E
- Requires tmux (cannot run in headless CI). Verified via logic trace.

## Files touched

- `crates/runie-core/src/model/state/turn_projections.rs` (queue pop in `apply_turn_started` — already landed)
- `crates/runie-tui/src/ui_actor.rs` (moved `agent_running = false` from `Done` to `TurnCompleted`)
- `crates/runie-tui/src/ui_actor_agent_handles.rs` (already wired `run_if_queued`)
- `crates/runie-tui/src/tests/agent_run_guard.rs` (updated tests)

## Validation

- `cargo check --workspace`: passes
- `cargo test --workspace`: 2802 passed, 0 failed
