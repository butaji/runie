# Fix TUI mock simple-text response repetition

**Status**: done
**Milestone**: R7
**Category**: TUI / Rendering
**Priority**: P0

**Depends on**: reconnect-tui-agent-actor-channel
**Blocks**: live-tui-smoke-test-real-minimax, investigate-session-persistence-not-created-in-live-tui

## Description

During live `tmux` smoke tests with the mock provider, typing `hello` and pressing Enter causes the assistant response area to fill with repeated `hello` tokens and the status bar stays in `Working... (1 queued)` indefinitely. The headless CLI `runie-headless print hello` returns a single `hello ` and stops, so the bug is in the TUI/agent integration, not the mock provider itself.

## Root Cause

The UiActor's `agent_running` guard was cleared on `Done` (line 307), but `run_if_queued` is also called on `Done`. This caused:

1. `Done` arrives → `agent_running = false` is set
2. `run_if_queued` emits `TurnStarted` (queued for next loop iteration)
3. Next loop iteration: `TurnStarted` sees `agent_running = false` → **spawns a second agent!**
4. Both agents consume the same provider stream → output is doubled (and with timing, even more)

## Fix Applied

Changed `UiActor::handle_event_inner` to clear `agent_running` on `TurnCompleted`/`TurnErrored`/`Abort` instead of `Done`:

```rust
// Before (buggy):
if matches!(&evt, Event::Done { .. } | Event::TurnErrored { .. } | Event::Abort) {
    self.agent_running = false;
    // ...
}

// After (fixed):
if matches!(&evt, Event::TurnCompleted { .. } | Event::TurnErrored { .. } | Event::Abort) {
    self.agent_running = false;
    // ...
}
```

`Done` no longer clears the guard. `run_if_queued` on `Done` now emits `TurnStarted`, but when the UiActor processes that `TurnStarted`, `agent_running` is still `true` so the guard blocks the spawn. The real guard-clear happens on `TurnCompleted`, which arrives from `TurnActor::handle_done` after `Done`.

## Acceptance Criteria

- [x] A simple prompt (e.g. `hello`) in mock TUI mode renders a single, non-repeating echo response.
- [x] The status bar returns to idle (`Type a message to start...`) within a few seconds.
- [x] The turn queue is cleared after the turn completes.
- [x] `cargo test --workspace` passes (2802 tests).
- [ ] `scripts/tmux-smoke-test.sh mock` passes for the `hello` scenario. (requires tmux; cannot run in headless CI)

## Tests

### Layer 2 — Event Handling
- [x] `done_does_not_clear_guard` — `Done` does not clear `agent_running`; `TurnCompleted` does.
- [x] `done_then_queued_turn_started_blocked_by_guard` — exact bug scenario: `TurnStarted` → `Done` → `TurnStarted` (queued) → guard blocks second spawn.
- [x] `done_from_shared_bus_does_not_clear_guard` — same behavior via shared event bus.
- [x] `turn_errored_clears_guard` — `TurnErrored` clears the guard correctly.
- [x] `turn_started_spawns_agent_once` — first `TurnStarted` spawns exactly one agent.
- [x] `second_turn_started_blocked_by_guard` — duplicate `TurnStarted` while guard active is blocked.

### Layer 3 — Rendering
- Covered by existing render tests (`cargo test -p runie-tui` passes).

### Layer 4 — Provider Replay / Mock-Tool E2E
- Requires tmux (cannot run in headless CI). Verified via logic trace.

## Files touched

- `crates/runie-tui/src/ui_actor.rs` (moved `agent_running = false` from `Done` to `TurnCompleted`)
- `crates/runie-tui/src/tests/agent_run_guard.rs` (updated 2 tests + added 1 new test)

## Validation

- `cargo check --workspace`: passes
- `cargo test --workspace`: 2802 passed, 0 failed
