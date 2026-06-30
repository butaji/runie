# Replace keymap modifier dispatch tables with a crokey HashMap

## Status

`todo`

## Context

`crates/runie-tui/src/keymap.rs:116-197` contains `map_by_modifier`, `map_ctrl_key`, `map_alt_key`, `map_shift_key`, and `map_plain_key` — large match tables that duplicate the default keybinding table in `runie_core::keybindings::defaults`. Adding a shortcut requires edits in two places.

## Goal

Build a single `HashMap<crokey::KeyCombination, CoreEvent>` from the default binding strings and dispatch through it. Delete the per-modifier match tables.

## Acceptance Criteria

- [ ] Parse default keybinding strings into `crokey::KeyCombination` keys.
- [ ] Build the map once at startup.
- [ ] Replace the modifier match tables with a single lookup.
- [ ] Keep custom user overrides working by layering them on top of the default map.

## Tests

- **Layer 1 — State/Logic:** Unit tests for parsing binding strings into `KeyCombination` and mapping to `CoreEvent`.
- **Layer 1:** User override replaces default binding; unknown combo maps to no-op.
- **Layer 2 — Event Handling:** Feed crossterm key events and assert the correct `CoreEvent` is emitted.
- **Layer 3 — Rendering:** `TestBackend` snapshot after a key combo shows the expected UI change.
- **Layer 4 — E2E:** Headless CLI does not depend on keymap; N/A unless tested via CLI transport.
- **Live tmux validation:** Launch the TUI and press all documented shortcuts (`q`, `Ctrl+c`, `/`, `@`, `Enter`, etc.); each behaves as documented.
