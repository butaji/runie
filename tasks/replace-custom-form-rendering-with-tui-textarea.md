# Replace custom form rendering with `tui-textarea`/`tui-input`

## Status

**done** — Superseded by `finish-replacing-custom-tui-widgets.md` which covers all custom TUI widgets including forms. Form inputs were replaced with `tui-input`.

## Original Description

`popups/panel/form.rs` is a custom 400-line form renderer. Replace editable fields with `tui-textarea`/`tui-input` and buttons with `ratatui::widgets::List`.

## Notes

- Input box replacement (`ui/input.rs`) is already done.
- Form rendering replacement is tracked in the canonical task.
- See `finish-replacing-custom-tui-widgets.md` for the current status.
