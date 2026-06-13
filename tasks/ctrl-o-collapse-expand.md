# Ctrl+O Collapse/Expand Feed Posts

**Status**: done
**Milestone**: R1
**Category**: Input & Commands
**Priority**: P0

## Description

The previous default `ctrl+shift+e` for collapsing/expanding feed posts did
not work reliably inside tmux because tmux (and many terminals without
extended-key reporting) cannot distinguish `Ctrl+Shift+E` from `Ctrl+E`.
This caused the key to be interpreted as `CursorEnd` instead of
`ToggleExpand`.

The fix remaps collapse/expand to `Ctrl+O`, which is unambiguous in tmux,
and keeps `Ctrl+E` as cursor-end. `Ctrl+Shift+E` is removed from the
default bindings entirely. `CopyLastResponse` moves from `Ctrl+O` to
`Ctrl+Shift+O`.

## Acceptance Criteria

- [x] `ctrl+o` default binding is `ToggleExpand`
- [x] `ctrl+e` default binding remains `CursorEnd`
- [x] `ctrl+shift+e` has no default binding
- [x] `ctrl+shift+o` default binding is `CopyLastResponse`
- [x] Bottom hint line shows `ctrl+o expand/collapse`
- [x] README keybindings table reflects the new defaults

## Tests

### Layer 1 — State/Logic
- [x] `crates/runie-core/src/keybindings.rs`: `ctrl_o_defaults_to_toggle_expand`,
  `ctrl_shift_e_has_no_default_binding`, `ctrl_shift_o_defaults_to_copy_last_response`
- [x] `crates/runie-core/src/tests/hints.rs`: `hint_shows_expand_hotkey_by_default`
  now asserts `ctrl+o`

### Layer 2 — Event Handling
- [x] `crates/runie-term/src/keymap/tests.rs`: `ctrl_o_converts_to_toggle_expand`,
  `ctrl_o_toggles_expand_state`, `ctrl_shift_e_is_unbound`,
  `ctrl_shift_e_lowercase_is_unbound_for_tmux`, `ctrl_shift_o_converts_to_copy_last_response`

### Layer 3 — Rendering
- [x] `crates/runie-tui/src/tests/render/input_box.rs`: hint line contains `ctrl+o`
- [x] `crates/runie-tui/src/tests/render/transient.rs`: default hints contain `ctrl+o`,
  transient overlay hides `ctrl+o`
- [x] Existing `runie-tui` collapse rendering tests still pass

### Layer 4 — Smoke (tmux)
- [x] `tmux_collapse_expand_test.sh`: starts the release binary in tmux,
  triggers a tool post, presses `Ctrl+O`, asserts the file list collapses
  behind a `[+]` summary, presses `Ctrl+O` again, asserts the list expands,
  and checks for panics/stuck timers.

## Verification

```bash
cargo test -p runie-core --lib
cargo test -p runie-term --bin runie
cargo test -p runie-tui toggle_expand
cargo build --release -p runie-term
./tmux_collapse_expand_test.sh
```
