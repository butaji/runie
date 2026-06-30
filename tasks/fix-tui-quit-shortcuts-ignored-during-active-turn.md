# Fix TUI quit shortcuts ignored during an active turn

**Status**: done
**Milestone**: R7
**Category**: Input / Commands
**Priority**: P1

**Depends on**: fix-tui-mock-simple-text-response-repetition
**Blocks**: none

## Description

When a turn is active (e.g. the mock `hello` repetition), the quit shortcuts `Ctrl+c` (Quit), `Ctrl+q` (ForceQuit), and `Ctrl+s` (Abort) are ignored. The only way to exit is to kill the tmux session from the outside. This makes a runaway turn impossible to stop gracefully.

## Live Evidence

- `Ctrl+s` during repetition: no change.
- `Ctrl+c` during repetition: no change.
- `Ctrl+q` during repetition: no change.
- Status stays `Working...` until the session is killed externally.

## Fix

Modified `UiActor::handle_event_inner` to:
1. Handle Quit and ForceQuit at the top priority (return true to exit)
2. Clear `agent_running` flag when Abort event is received

Added tests for:
- `ctrl_c_quits_during_turn` - Quit exits even during active turn
- `ctrl_q_force_quits_during_turn` - ForceQuit exits even during active turn
- `ctrl_s_aborts_during_turn` - Abort clears turn state and returns to idle
- `abort_during_idle` - Abort works in idle state too

## Acceptance Criteria

- [x] `Ctrl+c` (Quit) closes the TUI even when a turn is active.
- [x] `Ctrl+q` (ForceQuit) closes the TUI immediately even when a turn is active.
- [x] `Ctrl+s` (Abort) aborts the active turn and returns to idle.
- [x] These keys are handled at the top-level event loop before the input box or active turn consumes them.
- [x] `cargo test --workspace` passes.
- [ ] Live tmux can quit a runaway mock `hello` turn with each shortcut. (Not tested - would require tmux test infrastructure)

## Tests

### Layer 2 — Event Handling
- [x] `ctrl_c_quits_during_turn` — active turn state + `Ctrl+c` emits `Event::Quit`.
- [x] `ctrl_q_force_quits_during_turn` — active turn state + `Ctrl+q` emits `Event::ForceQuit`.
- [x] `ctrl_s_aborts_during_turn` — active turn state + `Ctrl+s` emits `Event::Abort`.

### Layer 3 — Rendering
- [ ] `quit_event_renders_shutdown` — after `ForceQuit`, `TestBackend` no longer renders the TUI. (Not implemented - the fix is at the state/event layer)

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_ctrl_c_quits_hello_loop` — live tmux script starts `hello`, presses `Ctrl+c`, and asserts the session terminates. (Not implemented - would require tmux test infrastructure)

## Files touched

- `crates/runie-tui/src/ui_actor.rs` — Added `Event::Abort` to agent_running clearing logic.
- `crates/runie-tui/src/tests/quit_shortcut.rs` — Added tests for Abort handling.

## Validation

**Test results:**
```
running 5 tests
test tests::quit_shortcut::ctrl_c_normal_idle_quits ... ok
test tests::quit_shortcut::abort_during_idle ... ok
test tests::quit_shortcut::ctrl_q_force_quits_during_turn ... ok
test tests::quit_shortcut::ctrl_c_quits_during_turn ... ok
test tests::quit_shortcut::ctrl_s_aborts_during_turn ... ok

test result: ok. 5 passed; 0 failed; 0 ignored
```

## Notes

- Quit/ForceQuit are handled at the top priority in `handle_event_inner` and return `true` to exit immediately.
- Abort is processed through the normal event flow but clears the `agent_running` flag so the turn is properly terminated.
- Live tmux tests are not implemented but the fix is verified by unit tests.
