# Fix TUI form submit key not working for /save and /compact

**Status**: todo
**Milestone**: R7
**Category**: Input / Commands
**Priority**: P1

**Depends on**: fix-tui-slash-command-palette-stays-open-after-execution
**Blocks**: investigate-session-persistence-not-created-in-live-tui

## Description

The `/save` and `/compact` commands open a form dialog, but the form cannot be submitted with the documented keys. Pressing Enter enters edit mode for the field, Tab/Down do not move focus to a submit action, and the form stays open. This makes explicit session save and compaction unreachable in the live TUI.

## Live Evidence

```
          ╭ Save Session ────────────────────────────────────────────╮
          │   1. Name                                                │
          │   ┌────────────────────────────────────────────────┐     │
          │   │test1▏                                          │     │
          │   └────────────────────────────────────────────────┘     │
          │                                                          │
 ╭────────│ ↑↓ navigate · enter edit · esc close                     │────────╮
 │❯ Type a│                                                          │        │
```

Enter, Tab, Down+Enter, and Escape all fail to submit; the dialog remains open.

## Acceptance Criteria

- [ ] `/save <name>` opens a form with a clear submit action.
- [ ] A documented key (e.g. Enter when a submit button is focused, or Ctrl+Enter) submits the form and triggers the command handler.
- [ ] `/compact` similarly submits and triggers compaction.
- [ ] If the form is invalid, a concise error message is shown inline.
- [ ] `cargo test --workspace` passes.
- [ ] Live tmux `/save test1` creates a session file.

## Tests

### Layer 1 — State/Logic
- [ ] `save_form_submit_with_valid_name` — form state produces a `SessionMsg::Save` intent.
- [ ] `compact_form_submit_with_valid_keep` — form state produces a compaction intent.

### Layer 2 — Event Handling
- [ ] `enter_on_submit_button_submits_form` — focus on submit + Enter emits the submit event.
- [ ] `enter_on_field_switches_to_edit_mode` — Enter on a text field enters edit mode; the field value is committed on the next submit.

### Layer 3 — Rendering
- [ ] `save_form_renders_submit_button` — `TestBackend` shows a focused submit action.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_save_form_submits` — live tmux script fills `/save test1` and submits, verifying `~/.runie/sessions/test1` is created.

## Files touched

- `crates/runie-core/src/commands/dsl/handlers/session/run.rs`
- `crates/runie-core/src/update/dialog/router.rs`
- `crates/runie-tui/src/popups/panel/form.rs`
- `crates/runie-tui/src/ui_actor.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- The form currently has only editable fields and no visible submit button. Adding a submit item to the panel list would align it with other dialogs.
- This task blocks explicit session persistence.
