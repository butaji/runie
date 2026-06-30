# Refine permission dialog key handling

**Status**: done
**Milestone**: R7
**Category**: Input / Commands
**Priority**: P1

**Depends on**: fix-tui-permission-dialog-key-input-routed-to-input
**Blocks**: wire-user-permission-rules-into-agent-gate

## Description

The permission dialog treats every key except `y`/`Y`/`a`/`A` as deny. Navigation keys (Esc, Enter, arrows) and unrelated characters accidentally deny the request, which is poor UX.

## Root Cause

`crates/runie-core/src/update/input/mod.rs` (`permission_input_event`) had a coarse match that mapped all non-allow keys to deny. The TUI `handle_input_event` did not consume navigation keys when the permission dialog was open.

## Acceptance Criteria

- [x] `y`/`Y` allows once; `a`/`A` allows always.
- [x] `n`/`N` explicitly denies.
- [x] Esc/Back/arrow keys are consumed as no-ops while the dialog is open (do not deny).
- [x] Keys that are not dialog actions are not routed to the input box.
- [x] `cargo test --workspace` passes.
- [ ] Live tmux permission dialog does not accidentally deny on arrow/Esc keys.

## Tests

### Layer 2 — Event Handling
- [x] `esc_during_permission_dialog_is_noop` — Esc while a permission request is open does not emit deny. (core + TUI)
- [x] `n_key_denies` — `n` explicitly denies. (core)
- [x] `backspace_during_permission_dialog_is_noop` — Backspace is a no-op. (core + TUI)
- [x] `newline_during_permission_dialog_is_noop` — Enter/Newline is a no-op. (core + TUI)
- [x] `cursor_keys_during_permission_dialog_are_noop` — Arrow keys are no-ops. (core + TUI)
- [x] `page_keys_during_permission_dialog_are_noop` — PageUp/PageDown are no-ops. (core + TUI)

### Layer 3 — Rendering
- [ ] `permission_dialog_shows_focus` — `TestBackend` highlights the focused choice.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_permission_esc_no_deny` — live tmux script opens the dialog, presses Esc, and asserts the dialog stays open.

## Files touched

- `crates/runie-core/src/update/input/mod.rs` — `permission_input_event` now consumes navigation keys as no-ops
- `crates/runie-tui/src/ui_actor.rs` — `handle_input_event` consumes navigation keys when permission dialog open
- `crates/runie-core/src/update/input/tests.rs` — new Layer 2 tests for no-op behavior
- `crates/runie-tui/src/tests/permission_dialog.rs` — new Layer 2 tests for navigation key no-ops

## Implementation

### Core (`permission_input_event`)
The function now checks for navigation/editing events before the deny fallback:

```rust
let consumed = matches!(
    event,
    crate::Event::Escape
        | crate::Event::Backspace
        | crate::Event::Newline
        | crate::Event::DeleteWord
        | ... // all navigation/editing keys
);
if consumed {
    return; // no-op: dialog stays open, no deny
}
```

### TUI (`handle_input_event`)
Added a guard at the top of `handle_input_event` that consumes navigation keys when a permission is pending:

```rust
if self.state.permission_request_opt().is_some()
    && is_navigation_or_editing_event(evt)
{
    return; // no-op
}
```

Helper `is_navigation_or_editing_event` lists all keys that should be silently consumed.
