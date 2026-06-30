# Fix TUI permission dialog keys routed to input box

**Status**: todo
**Milestone**: R7
**Category**: TUI / Rendering
**Priority**: P1

**Depends on**: fix-tui-mock-simple-text-response-repetition
**Blocks**: live-tui-smoke-test-real-minimax

## Description

When a native tool (e.g. `bash`) requests permission, the permission dialog is rendered but the `y`/`n`/`a` keys are captured by the bottom input box instead of the dialog. The dialog therefore cannot be answered, and the tool call hangs.

## Live Evidence

```
          ╭ Permission Required ─────────────────────────────────────╮
          │ Tool: bash                                               │
          │ Input: {  "command": "echo hi"}                          │
          │ [y] Allow   [n] Deny   [a] Always allow                  │
          ╰──────────────────────────────────────────────────────────╯

  ⠧ Working... 0.0s (1 queued)                         ...
 ╭────────────────────────────────────────────────────────────────────────────╮
 │❯ y                                                                         │
 ╰───────────────────────────────────────────────────────────────── mock/echo ╯
```

Pressing `y` appears in the input line; the dialog does not advance.

## Acceptance Criteria

- [ ] While the permission dialog is open, `y`, `n`, and `a` are consumed by the dialog and do not appear in the input box.
- [ ] `y` grants the pending tool call once.
- [ ] `n` denies the pending tool call once.
- [ ] `a` grants the pending tool call and updates the trust rule to always allow.
- [ ] `cargo test --workspace` passes.
- [ ] Live tmux `native tool` scenario can allow the tool and see its output.

## Tests

### Layer 2 — Event Handling
- [ ] `permission_dialog_y_grants` — dialog-open state + `y` key emits `PermissionMsg::Grant` and clears the dialog.
- [ ] `permission_dialog_n_denies` — dialog-open state + `n` key emits `PermissionMsg::Deny` and clears the dialog.
- [ ] `permission_dialog_keys_not_sent_to_input` — assert the input buffer is unchanged after `y`/`n`/`a` while the dialog is open.

### Layer 3 — Rendering
- [ ] `permission_dialog_renders_options` — `TestBackend` snapshot shows the dialog with `[y] Allow [n] Deny [a] Always allow`.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_native_tool_allow` — live tmux script presses `y` and verifies the bash output `hi` appears.

## Files touched

- `crates/runie-tui/src/ui_actor.rs`
- `crates/runie-core/src/update/dialog/router.rs`
- `crates/runie-core/src/actors/permission/ractor_permission.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- The dialog may not actually have keyboard focus; the input box is still the active receiver.
- Permission handling works in unit tests, so this is a TUI focus-routing bug.
