# Replace keymap modifier dispatch tables with a crokey HashMap

## Status

`done`

## Context

`crates/runie-tui/src/keymap.rs:116-197` contains `map_by_modifier`, `map_ctrl_key`, `map_alt_key`, `map_shift_key`, and `map_plain_key` — large match tables that duplicate the default keybinding table in `runie_core::keybindings::defaults`. Adding a shortcut requires edits in two places.

## Goal

Build a single `HashMap<crokey::KeyCombination, CoreEvent>` from the default binding strings and dispatch through it. Delete the per-modifier match tables.

**Design impact:** No change to TUI element design or composition. Only the internal key-event dispatch behavior changes; every shortcut must produce the same visible result as before.

## Acceptance Criteria

- [x] Parse default keybinding strings into `crokey::KeyCombination` keys.
- [x] Build the map once at startup.
- [x] Replace the modifier match tables with a single lookup.
- [x] Keep custom user overrides working by layering them on top of the default map.

## Tests

- **Layer 1 — State/Logic:** Unit tests for parsing binding strings into `KeyCombination` and mapping to `CoreEvent`.
- **Layer 1:** User override replaces default binding; unknown combo maps to no-op.
- **Layer 2 — Event Handling:** Feed crossterm key events and assert the correct `CoreEvent` is emitted.
- **Layer 3 — Rendering:** `TestBackend` snapshot after a key combo shows the expected UI change.
- **Layer 4 — E2E:** Headless CLI does not depend on keymap; N/A unless tested via CLI transport.
- **Live tmux testing session (required):** Launch the TUI and press all documented shortcuts (`q`, `Ctrl+c`, `/`, `@`, `Enter`, etc.); each behaves as documented.

## Implementation Notes

The keymap now uses:
- `DEFAULT_MAP`: `LazyLock<HashMap<String, CoreEvent>>` built from `keybindings::default_keybindings()` once at startup
- `map_key_event()`: Single lookup function with priority: 1) user bindings override, 2) default map, 3) plain key fallback
- `map_plain_key()`: Minimal fallback for unhandled keys (Esc, Tab, chars, navigation)

The old per-modifier tables (`map_by_modifier`, `map_ctrl_key`, etc.) have been deleted.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
