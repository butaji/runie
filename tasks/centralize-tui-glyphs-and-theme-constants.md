# Centralize TUI glyphs and theme constants

## Status

`todo`

## Description

Spinner frames, checkboxes (`[x]`/`[ ]`), arrows, ellipsis, separators, and braille arrays are duplicated and hard-coded across `ui.rs`, `status_bar.rs`, `message/support.rs`, `popups/panel/form.rs`, `popups/panel/list.rs`, and `dialog/builders/palette.rs`.

## Acceptance criteria

1. **Unit tests** — All glyphs live in `runie-tui::theme::glyphs` and are imported where needed.
2. **E2E tests** — `TestBackend` rendering snapshots match before and after.
3. **Live tmux tests** — Open dialogs, checkboxes, and spinners in tmux and verify visuals.

## Tests

### Unit tests
- Glyph constants are defined once and reused.

### E2E tests
- Buffer assertions for checkbox/spinner/separator rendering.

### Live tmux tests
- Open the command palette and a settings form; inspect symbols.
