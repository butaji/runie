# Use `textwrap` for blockquote word wrap

## Status

`todo`

## Description

`message/support.rs` reimplements word-wrapping by character width. Use `textwrap` with display-width options.

## Acceptance criteria

1. **Unit tests** — Wrapped output matches the custom implementation for sample blockquotes.
2. **E2E tests** — Rendering snapshots are unchanged.
3. **Live tmux tests** — View a message containing blockquotes in tmux.

## Tests

### Unit tests
- Blockquote wrapping for various widths.

### E2E tests
- `TestBackend` buffer assertions for blockquote messages.

### Live tmux tests
- Display a message with quoted text.
