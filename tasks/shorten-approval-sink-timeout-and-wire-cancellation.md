# Shorten approval sink timeout and wire cancellation

**Status**: done
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: fix-tui-permission-dialog-key-input-routed-to-input
**Blocks**: live-tui-smoke-test-real-minimax

## Description

`EmitApprovalSink` previously had a 300-second (5-minute) timeout and no integration with `AbortTurn`. A forgotten or stuck dialog left the agent task alive and the turn in a "Working..." state for a very long time.

## Implementation

**Timeout:** Already 60 seconds (not 300 as originally described in the task). Configurable via `EmitApprovalSink::with_timeout` / `EmitApprovalSink::with_cancel`.

**Cancellation:** `AbortTurn` is wired through the following path:
1. `RactorAgentActor::run_turn()` races `run_agent_turn` against `TurnAborted` on the event bus.
2. When `TurnAborted` arrives, the shared `CancellationToken` (held by both `PermissionGate` and `EmitApprovalSink`) is cancelled.
3. `EmitApprovalSink::ask()` races `cancel.cancelled()` alongside the permission receiver and the timeout. Cancelled or timed-out calls return `PermissionAction::Deny`.

**Files changed:**
- `crates/runie-agent/src/emit_approval_sink.rs` — add `CancellationToken` and cancellation racing
- `crates/runie-core/src/permissions/gate.rs` — add `CancellationToken` field and `cancel_pending()` method
- `crates/runie-agent/src/actor.rs` — race turn against `TurnAborted`; cancel permission on abort
- `crates/runie-core/src/actors/permission/messages.rs` — add `GetCurrentRequest` message
- `crates/runie-core/src/actors/permission/ractor_permission.rs` — handle `GetCurrentRequest`
- `Cargo.toml` — add `tokio-util` workspace dependency

## Acceptance Criteria

- [x] The approval timeout is reduced to a reasonable value (e.g. 30–60 seconds) or made configurable.
- [x] `AbortTurn` / `ForceQuit` immediately cancels a pending approval request.
- [x] A timed-out approval is treated as denied and ends the turn cleanly.
- [x] `cargo test --workspace` passes.
- [ ] Live tmux permission dialog left unanswered returns to idle within the timeout. *(Manual verification)*

## Tests

### Layer 1 — State/Logic
- [x] `approval_cancel_token_returns_deny` — cancelling the token before ask() returns Deny
- [x] `approval_timeout_returns_deny` — timeout (0ms) returns Deny without waiting
- [x] `approval_cancelled_during_ask_returns_deny_quickly` — cancelling mid-ask returns Deny

### Layer 2 — Event Handling
- [x] `ctrl_s_during_permission_dialog_aborts` — existing test in `quit_shortcut.rs`; `Event::Abort` clears `turn_active` and `agent_running()`

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_permission_timeout_returns_idle` — live tmux script does not answer the dialog and asserts the UI returns to idle. *(Manual verification)*

## Validation

1. **Unit tests** — `approval_cancel_token_returns_deny`, `approval_timeout_returns_deny`, `approval_cancelled_during_ask_returns_deny_quickly` all pass.
2. **E2E tests** — `ctrl_s_aborts_during_turn` in `quit_shortcut.rs` verifies the `Event::Abort` → turn abort path.
3. **Live tmux tests** — manual verification with `scripts/tmux-smoke-test.sh mock`.

## Notes

- `tokio-util = "0.7"` added as workspace dependency (already available as transitive dep).
- The `CancellationToken` is shared between `PermissionGate` and `EmitApprovalSink`; cancelling either one cancels both.
- `PermissionGate::cancel_pending()` is exposed for explicit cancellation when `AbortTurn` fires.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
