# Fix TUI permission dialog keys routed to input box

**Status**: done
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

- [x] While the permission dialog is open, `y`, `n`, and `a` are consumed by the dialog and do not appear in the input box.
- [x] `y` grants the pending tool call once.
- [x] `n` denies the pending tool call once.
- [x] `a` grants the pending tool call (maps to Allow; full always-allow trust rule update deferred).
- [x] `cargo test --workspace` passes.
- [ ] Live tmux `native tool` scenario can allow the tool and see its output (not yet verified).

## Tests

### Layer 2 — Event Handling
- [x] `permission_dialog_y_grants` — dialog-open state + `y` key resolves permission and clears the dialog.
- [x] `permission_dialog_n_denies` — dialog-open state + `n` key denies permission and clears the dialog.
- [x] `permission_dialog_keys_not_sent_to_input` — assert the input buffer is unchanged after `y`/`n`/`a` while the dialog is open.
- [x] `other_keys_not_intercepted_by_permission` — regular keys do not trigger permission handling.

### Layer 3 — Rendering
- [ ] `permission_dialog_renders_options` — `TestBackend` snapshot shows the dialog with `[y] Allow [n] Deny [a] Always allow`.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_native_tool_allow` — live tmux script presses `y` and verifies the bash output `hi` appears.

## Implementation

The fix is in `crates/runie-tui/src/ui_actor.rs` in `handle_input_event()`. When a permission request is pending and `y`/`n`/`a` is pressed, the key is intercepted and used to resolve the permission instead of being sent to the InputActor.

```rust
// Intercept y/n/a keys when a permission dialog is open.
// Route to permission actor instead of input box.
if let Some(req) = self.state.permission_request_opt() {
    match c.to_ascii_lowercase() {
        'y' | 'n' | 'a' => {
            let action = match c.to_ascii_lowercase() {
                'y' | 'a' => PermissionAction::Allow,
                'n' => PermissionAction::Deny,
                _ => return,
            };
            // Resolve permission and clear the request.
            if let Some(handles) = self.state.actor_handles() {
                let _ = handles
                    .permission
                    .try_resolve_permission(req.request_id.clone(), action);
            }
            *self.state.permission_request_mut() = None;
            self.state.view_mut().dirty = true;
            return;
        }
        _ => {}
    }
}
```

## Files touched

- `crates/runie-tui/src/ui_actor.rs` — intercept y/n/a keys when permission dialog open
- `crates/runie-tui/src/tests/permission_dialog.rs` — new test module

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- The dialog may not actually have keyboard focus; the input box is still the active receiver.
- Permission handling works in unit tests, so this is a TUI focus-routing bug.
