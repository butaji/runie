# Use `textwrap` for blockquote word wrap

## Status

`done` — `textwrap` is already used in `wrap_styled_spans_for_blockquote` (support.rs:197-253).

## Description

`message/support.rs` reimplements word-wrapping by character width. Use `textwrap` with display-width options.

### Implementation

`wrap_styled_spans_for_blockquote` in `crates/runie-tui/src/message/support.rs` uses `textwrap::wrap` for both single-span and multi-span cases:
- Single-span: direct `textwrap::wrap` call (line 212)
- Multi-span: `textwrap::wrap` for breaking long spans (line 238)
- Tests pass: `blockquote_renders_inline_styles`

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
