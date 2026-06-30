# Fix TUI Ctrl+o expand/collapse has no visible effect

**Status**: todo
**Milestone**: R7
**Category**: TUI / Rendering
**Priority**: P2

**Depends on**: fix-tui-mock-simple-text-response-repetition
**Blocks**: none

## Description

The bottom key legend advertises `ctrl+o expand/collapse`, but pressing `Ctrl+o` after a tool turn produces no visible change in the assistant message panel.

## Live Evidence

After `list files` completed, pressing `Ctrl+o` left the pane identical:
```
  config.schema.json
  crates/
  ...

  →  ◐ 0.1s

  Turn completed in 0.0s

  ⠹ Working... 0.0s (1 queued)                         ...
```

## Acceptance Criteria

- [ ] `Ctrl+o` toggles the expanded/collapsed view of the current assistant message or tool block.
- [ ] The visual state change is observable in `TestBackend` snapshots.
- [ ] The key is not swallowed by the input box or other dialogs.
- [ ] `cargo test --workspace` passes.
- [ ] Live tmux `list files` followed by `Ctrl+o` shows a visible change.

## Tests

### Layer 1 — State/Logic
- [ ] `toggle_expand_flips_flag` — `Event::ToggleExpand` flips the expand flag in view state.

### Layer 2 — Event Handling
- [ ] `ctrl_o_emits_toggle_expand` — `Ctrl+o` key event maps to `Event::ToggleExpand` even during a turn.

### Layer 3 — Rendering
- [ ] `expanded_message_renders_more_lines` — `TestBackend` snapshot differs between collapsed and expanded states.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_ctrl_o_toggles_expand` — live tmux script runs `list files`, presses `Ctrl+o`, and asserts the captured pane changes.

## Files touched

- `crates/runie-tui/src/ui_actor.rs`
- `crates/runie-tui/src/state.rs`
- `crates/runie-tui/src/ui.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- This may be related to the status/queue not clearing after a turn; expand/collapse may only apply to a selected message that is not focused.
- Fix after the turn-completion bug so the UI is in a known state.
