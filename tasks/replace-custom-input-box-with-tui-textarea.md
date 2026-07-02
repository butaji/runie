# Replace custom input box with `tui-textarea`

## Status

`todo`

## Description

`ui/input.rs` is a custom multi-line input box. Replace it with `tui-textarea` or `tui-input`.

## Acceptance criteria

1. **Unit tests** — Cursor, line count, scrolling, and submit behavior match.
2. **E2E tests** — Input events produce the same state.
3. **Live tmux tests** — Type multi-line input, use Enter/Shift+Enter, and submit in tmux.

## Tests

### Unit tests
- Cursor movement, line insertion, scrolling.

### E2E tests
- Key events update input state correctly.

### Live tmux tests
- Compose and submit multi-line messages.
