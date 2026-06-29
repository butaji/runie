# Replace TUI keymap combo stringification with `crokey`

**Status**: done
**Milestone**: R6
**Category**: Input / Commands
**Priority**: P1

**Depends on**: replace-custom-helpers-with-crates
**Blocks**: none

## Description

`crates/runie-tui/src/keymap.rs` builds strings like `"ctrl+c"` manually in `key_event_to_combo`. Replace this with `crokey` (or crossterm `Display`/`FromStr`) so combo strings round-trip with `KeyEvent`.

## Acceptance Criteria

- [x] Replace manual combo stringification with `crokey` backed by `KeyCombinationFormat`.
- [x] Ensure all existing keybindings still parse/display correctly.
- [x] Preserve `Esc` → "escape" and `BackTab` → "shift+tab" legacy aliases.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `key_combo_round_trip_ctrl_c` — `ctrl-c` (crokey parse) → "ctrl+c" (output).
- [x] `key_combo_round_trip_alt_enter` — `alt-enter` → "alt+enter".
- [x] `key_combo_round_trip_shift_tab` — `shift-backtab` → "shift+tab" (legacy alias).
- [x] `key_combo_round_trip_uppercase_modifiers` — `Ctrl-Shift-M` → "ctrl+shift+m".
- [x] `key_event_to_combo_ctrl_c` / `alt_enter` / `shift_enter` / `plain_escape` — unchanged output.

### Layer 2 — Event Handling
- [x] `keybinding_event_maps_to_action` — `Ctrl+Shift+O` (uppercase) triggers `CopyLastResponse`.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `crates/runie-tui/src/keymap.rs` — `key_event_to_combo` now backed by `crokey::KeyCombinationFormat`; two legacy aliases preserved (`Esc` → "escape", `BackTab` → "shift+tab").
- `crates/runie-tui/Cargo.toml` — added `crokey = "1.4.0"`, `once_cell = "1.21.4"`.
- `crates/runie-tui/src/keymap/tests/combos.rs` — added 4 round-trip tests.
- `crates/runie-tui/src/keymap/tests/basic.rs` — added `keybinding_event_maps_to_action`.

## Implementation Notes

- `crokey` uses `-` as the separator; the output is lowercased and `-`→`+` to match the existing config format.
- `KeyCombinationFormat::default().with_lowercase_modifiers()` provides the formatter backbone.
- Two special cases are preserved: `Esc` → "escape" (crokey formats as "Esc") and `BackTab` → "shift+tab" (the binding table uses this legacy alias).
