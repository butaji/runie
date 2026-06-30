# Investigate session persistence not created during live TUI runs

**Status**: todo
**Milestone**: R7
**Category**: Sessions
**Priority**: P1

**Depends on**: fix-tui-mock-simple-text-response-repetition
**Blocks**: none

## Description

During live `tmux` smoke tests, no `~/.runie/sessions/` directory is created and no session file appears after a completed mock turn. Explicit save via `/save` is the intended persistence path, but the `/save` form in the TUI cannot be submitted with the documented keys, so sessions cannot be persisted at all in the live TUI.

## Live Evidence

- After a completed `list files` turn, `~/.runie/sessions/` does not exist.
- `/save test1` opens a form with the name pre-filled, but Enter, Tab, Down+Enter, and Escape do not submit it.
- `/sessions` reports `No saved sessions. Use /save name to create one.`

## Acceptance Criteria

- [ ] `/save <name>` in the live TUI creates a session file under `~/.runie/sessions/` (or the configured OS data dir).
- [ ] `/sessions` lists the saved session after a successful save.
- [ ] `/load <name>` restores the saved session messages.
- [ ] `cargo test --workspace` passes, including the existing `save_after_completed_turn_creates_session_file` regression test.
- [ ] Live tmux verification shows a saved session is created and reloadable.

## Tests

### Layer 1 — State/Logic
- [ ] `save_form_submit_creates_session_file` — drive the form state machine to completion and assert `SessionStore::append_batch` is called.
- [ ] `sessions_command_lists_saved_sessions` — populate the store, run `/sessions`, and assert the returned list contains the saved name.

### Layer 2 — Event Handling
- [ ] `save_form_keys_submit` — simulate the key sequence that opens the `/save` form and submits it, verifying the correct `SessionMsg::Save` event is emitted.

### Layer 3 — Rendering
- [ ] `save_form_renders_submit_button` — assert the save form renders a focused submit action and that Enter/Space activates it.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_save_and_load_session` — live tmux script runs `/save foo`, quits, restarts, runs `/load foo`, and verifies the previous messages appear.

## Files touched

- `crates/runie-core/src/commands/dsl/handlers/session/run.rs`
- `crates/runie-core/src/update/dialog/router.rs`
- `crates/runie-tui/src/ui_actor.rs`
- `crates/runie-core/src/session/replay.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- The previous investigation concluded that persistence is explicit via `/save`, which is the intended design. The new finding is that the `/save` form is not submittable in the live TUI, making persistence unreachable.
- Once `/save` works, decide whether to also auto-save on graceful TUI exit; that is out of scope for this task.
