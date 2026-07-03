# Centralize TUI popup layout constants

## Status

`done`

## Implementation

Created `crates/runie-tui/src/popups/layout_constants.rs` with centralized constants:
- `POPUP_WIDTH = 60`
- `POPUP_HEIGHT = 18`
- `POPUP_MIN_WIDTH = 20`
- `POPUP_MIN_HEIGHT = 6`
- `PATH_DISPLAY_COUNT = 8`
- `PATH_POPUP_BORDER = 4`
- `HOTKEY_AREA_HEIGHT = 2`

Updated references in:
- `crates/runie-tui/src/popups.rs`
- `crates/runie-tui/src/popups/panel/mod.rs`

## Context

`todo`

## Description

Popup sizes, margins, max suggestions, and input-box layout math are scattered in `popups.rs`, `popups/panel/form.rs`, `popups/panel/list.rs`, `ui.rs`, `ui/input.rs`, and `core/layout.rs`.

## Acceptance criteria

1. **Unit tests** — Layout calculations use named constants; computed rectangles match old behavior.
2. **E2E tests** — Dialog snapshots are unchanged.
3. **Live tmux tests** — Resize the tmux window while dialogs are open and verify layout stays correct.

## Tests

### Unit tests
- Popup rectangle calculations with sample terminal sizes.

### E2E tests
- `TestBackend` snapshots for palette, form, and list popups.

### Live tmux tests
- Open palette, form, and file picker and resize the pane.
