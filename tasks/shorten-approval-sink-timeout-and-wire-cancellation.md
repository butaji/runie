# Shorten approval sink timeout and wire cancellation

**Status**: todo
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: fix-tui-permission-dialog-key-input-routed-to-input
**Blocks**: live-tui-smoke-test-real-minimax

## Description

`EmitApprovalSink` waits 5 minutes (`Duration::from_secs(300)`) for the user to resolve a permission request. A forgotten or stuck dialog leaves the agent task alive and the turn in a “Working...” state for a very long time.

## Root Cause

The timeout is hardcoded in `crates/runie-agent/src/emit_approval_sink.rs` and the cancellation path is not tied to `AbortTurn`.

## Acceptance Criteria

- [ ] The approval timeout is reduced to a reasonable value (e.g. 30–60 seconds) or made configurable.
- [ ] `AbortTurn` / `ForceQuit` immediately cancels a pending approval request.
- [ ] A timed-out approval is treated as denied and ends the turn cleanly.
- [ ] `cargo test --workspace` passes.
- [ ] Live tmux permission dialog left unanswered returns to idle within the timeout.

## Tests

### Layer 1 — State/Logic
- [ ] `approval_timeout_emits_deny` — simulate no user response and assert the result is `Denied` after the timeout.
- [ ] `abort_cancels_pending_approval` — `AbortTurn` terminates the sink future.

### Layer 2 — Event Handling
- [ ] `ctrl_s_during_permission_dialog_aborts` — `Ctrl+s` while a dialog is open emits abort and denies the request.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_permission_timeout_returns_idle` — live tmux script does not answer the dialog and asserts the UI returns to idle.

## Files touched

- `crates/runie-agent/src/emit_approval_sink.rs`
- `crates/runie-core/src/actors/permission/ractor_permission.rs`
- `crates/runie-core/src/actors/turn/ractor_turn.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- This is related to the permission dialog focus bug; once the dialog can be answered, the timeout becomes the next reliability issue.
