# Fix TUI quit shortcuts ignored during an active turn

**Status**: todo
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

## Acceptance Criteria

- [ ] `Ctrl+c` (Quit) closes the TUI even when a turn is active.
- [ ] `Ctrl+q` (ForceQuit) closes the TUI immediately even when a turn is active.
- [ ] `Ctrl+s` (Abort) aborts the active turn and returns to idle.
- [ ] These keys are handled at the top-level event loop before the input box or active turn consumes them.
- [ ] `cargo test --workspace` passes.
- [ ] Live tmux can quit a runaway mock `hello` turn with each shortcut.

## Tests

### Layer 2 — Event Handling
- [ ] `ctrl_c_quits_during_turn` — active turn state + `Ctrl+c` emits `Event::Quit`.
- [ ] `ctrl_q_force_quits_during_turn` — active turn state + `Ctrl+q` emits `Event::ForceQuit`.
- [ ] `ctrl_s_aborts_during_turn` — active turn state + `Ctrl+s` emits `Event::Abort`.

### Layer 3 — Rendering
- [ ] `quit_event_renders_shutdown` — after `ForceQuit`, `TestBackend` no longer renders the TUI.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_ctrl_c_quits_hello_loop` — live tmux script starts `hello`, presses `Ctrl+c`, and asserts the session terminates.

## Files touched

- `crates/runie-tui/src/ui_actor.rs`
- `crates/runie-tui/src/keymap.rs`
- `crates/runie-core/src/actors/turn/ractor_turn.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- These shortcuts may be routed to the input box or swallowed by the streaming update loop. They need priority handling in the main event dispatcher.
- Abort should cancel the provider stream; Quit/ForceQuit should exit the process.
