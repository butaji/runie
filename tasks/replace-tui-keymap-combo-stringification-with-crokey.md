# Replace TUI keymap combo stringification with `crokey`

**Status**: todo
**Milestone**: R6
**Category": Input / Commands
**Priority": P1

**Depends on**: replace-custom-helpers-with-crates
**Blocks**: none

## Description

`crates/runie-tui/src/keymap.rs` builds strings like `"ctrl+c"` manually in `key_event_to_combo`. Replace this with `crokey` (or crossterm `Display`/`FromStr`) so combo strings round-trip with `KeyEvent`.

## Acceptance Criteria

- [ ] Replace manual combo stringification with `crokey` or crossterm `Display`/`FromStr`.
- [ ] Ensure all existing keybindings still parse/display correctly.
- [ ] Delete `key_event_to_combo` and related helpers.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `key_combo_round_trip` — `ctrl+c`, `alt+enter`, `shift+tab` parse and display correctly.

### Layer 2 — Event Handling
- [ ] `keybinding_event_maps_to_action` — a parsed combo triggers the right action.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-tui/src/keymap.rs`
- `crates/runie-core/src/keybindings/mod.rs`
- `crates/runie-tui/Cargo.toml`

## Notes

- Coordinate with `replace-custom-helpers-with-crates.md` for core keybindings.
