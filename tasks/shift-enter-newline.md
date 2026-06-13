# Shift+Enter Inserts Newline

**Status**: done
**Milestone**: R1
**Category**: Input & Commands
**Priority**: P1
**Depends on**: terminal-detection

## Description

Runie's input box already maps `Ctrl+J` to a newline. Users expect the
same behaviour from `Shift+Enter` on terminals that support reporting
modified keys. This task makes `Shift+Enter` reliably produce a newline
and adds smoke coverage for the enhanced-key path.

## What Was Done

- [x] Extended the kitty keyboard protocol flags pushed at startup from
  `DISAMBIGUATE_ESCAPE_CODES | REPORT_ALL_KEYS_AS_ESCAPE_CODES` to also
  include `REPORT_EVENT_TYPES | REPORT_ALTERNATE_KEYS`.
- [x] Added the xterm `modifyOtherKeys` level-2 sequence (`CSI > 4; 2 m`)
  alongside the kitty protocol, and reset it (`CSI > 4; 0 m`) on shutdown.
  This is the most compatible way to make terminals report Shift+Enter
  as a CSI sequence.
- [x] Updated `crates/runie-term/src/tests/terminal_setup.rs` to expect
  both the kitty and xterm push sequences, plus reset sequences.
- [x] Removed the accidental bare `y` shortcut in `push_input` that
  conflicted with typing `y` as message content; `Ctrl+O` remains the
  default copy-last-response binding.
- [x] Migrated all bash smoke tests from `tests/smoke/*.sh` into Rust
  e2e tests under `crates/runie-term/tests/e2e/smoke_*.rs` and deleted
  the `tests/smoke` directory.

## Acceptance Criteria

- [x] `Shift+Enter` (when reported by the terminal) inserts `\n` in the
  input box, identical to `Ctrl+J`.
- [x] The bare `y` key no longer triggers copy when the input box is empty.
- [x] All pre-existing queue tests pass again.

## Tests

### Layer 1 — State/Logic
- [x] `crates/runie-term/src/tests/terminal_setup.rs` verifies the
  enhanced-keyboard push sequence is emitted.

### Layer 2 — Event Handling
- [x] `crates/runie-term/src/keymap/tests.rs` already contains:
  - `shift_enter_converts_to_newline`
  - `shift_f3_converts_to_newline_for_tmux_shift_enter`
  - `f3_without_shift_converts_to_newline_for_tmux_compat`
  - `ctrl_j_converts_to_newline`

### Layer 3 — Rendering
- [x] `crates/runie-core/src/tests/input_scroll.rs` and
  `crates/runie-core/src/tests/input_grapheme.rs` cover multiline
  input rendering.

### Layer 4 — Smoke
- [x] `crates/runie-term/tests/e2e/smoke_shift_enter.rs` exercises the
  full binary through a PTY with a raw kitty-protocol Shift+Enter
  sequence.
- [x] All former bash smoke tests are now Rust e2e tests:
  `smoke_basic`, `smoke_keyboard_interrupt`, `smoke_long_conversation`,
  `smoke_rapid_submit`, `smoke_resize_stress`,
  `smoke_session_persistence`, `smoke_session_tree`,
  `smoke_tab_completion`.

## Verification

```bash
# Unit tests
cargo test -p runie-term terminal_setup
cargo test -p runie-term keymap
cargo test -p runie-core --lib

# Smoke tests (ignored by default; require release binary)
cargo test -p runie-term --test e2e smoke_ -- --ignored
```
