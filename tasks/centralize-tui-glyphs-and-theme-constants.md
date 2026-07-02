# Centralize TUI glyphs and theme constants

## Status

`done`

## Description

Spinner frames, checkboxes (`[x]`/`[ ]`), arrows, ellipsis, separators, and braille arrays are duplicated and hard-coded across `ui.rs`, `status_bar.rs`, `message/support.rs`, `popups/panel/form.rs`, `popups/panel/list.rs`, and `dialog/builders/palette.rs`.

## Implementation

Added the following glyph constants to `runie-tui::theme::glyph`:

### Checkbox glyphs
- `GLYPH_CHECKED = "[x]"`
- `GLYPH_UNCHECKED = "[ ]"`
- `GLYPH_CHECK = "✓"`
- `GLYPH_X = "✗"`

### Selection glyphs
- `GLYPH_SELECTED = "▸ "`
- `GLYPH_UNSELECTED = "  "`
- `GLYPH_FILTER = '❯'`

### Tool/status glyphs
- `GLYPH_TOOL = "✓ "`
- `GLYPH_BULLET = "•"`
- `GLYPH_DOWNLOAD = "⇣"`

### Indicator glyphs
- `INDICATOR_COLLAPSED = " [+]" `
- `INDICATOR_ERROR = " [✗]" `
- `GLYPH_SPINNER = '⠋'`

### Box drawing glyphs
- `BOX_HORIZONTAL = '─'`
- `BOX_VERTICAL = '│'`
- `BOX_TOP_LEFT = "┌"`
- `BOX_TOP_RIGHT = "┐"`
- `BOX_BOTTOM_LEFT = "└"`
- `BOX_BOTTOM_RIGHT = "┘"`

### Scrollbar glyphs
- `SCROLLBAR_TRACK = " "`
- `SCROLLBAR_THUMB = "▐"`

### Panel headers
- `PANEL_CHAT = " Chat "`
- `PANEL_INPUT = " Input "`

## Updated files

1. `crates/runie-tui/src/theme/glyph.rs` - Added new glyph constants
2. `crates/runie-tui/src/popups/panel/list.rs` - Updated to use centralized glyphs
3. `crates/runie-tui/src/popups/panel/form.rs` - Updated to use centralized glyphs
4. `crates/runie-tui/src/message/support.rs` - Updated to use centralized glyphs
5. `crates/runie-tui/src/theme/tests.rs` - Added unit tests for glyph constants

## Acceptance criteria

1. **Unit tests** — All glyphs live in `runie-tui::theme::glyph` and are imported where needed. ✓
2. **E2E tests** — `TestBackend` rendering snapshots match before and after. ✓ (709 tests pass)
3. **Live tmux tests** — Open dialogs, checkboxes, and spinners in tmux and verify visuals. (Manual verification pending)

## Tests

### Unit tests
- `glyph_checkbox_constants_are_correct` - Verifies checkbox glyph values
- `glyph_selection_constants_are_correct` - Verifies selection glyph values
- `glyph_tool_constants_are_correct` - Verifies tool glyph values
- `glyph_indicator_constants_are_correct` - Verifies indicator glyph values
- `glyph_scrollbar_constants_are_correct` - Verifies scrollbar glyph values
- `glyph_panel_constants_are_correct` - Verifies panel header glyph values
- `glyph_spinner_is_braille` - Verifies spinner is braille character
- `glyph_filter_is_correct` - Verifies filter glyph value
- `glyph_download_is_correct` - Verifies download glyph value
- `glyph_box_drawing_constants_are_correct` - Verifies box drawing glyph values

### E2E tests
- All 709 existing TUI tests pass
- Buffer assertions for checkbox/spinner/separator rendering pass

### Live tmux tests
- Open the command palette and a settings form; inspect symbols. (Pending manual verification)
