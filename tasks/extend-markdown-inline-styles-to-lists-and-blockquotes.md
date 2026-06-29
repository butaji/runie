# Extend markdown inline styles to lists and blockquotes

**Status**: todo
**Milestone**: R7
**Category**: Core / State
**Priority**: P2

**Depends on**: unify-markdown-block-parsing-on-pulldown-cmark-events
**Blocks**: none

## Description

`crates/runie-core/src/markdown/blocks.rs` accumulates plain text only for `List` and `Blockquote` states. Extend `BlockParser` to emit styled inline spans for list items and blockquotes, then update layout and renderers.

## Acceptance Criteria

- [ ] `List` state preserves inline styles (bold, italic, code, links).
- [ ] `Blockquote` state preserves inline styles.
- [ ] `layout.rs` line counts and TUI renderers honor them.
- [ ] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [ ] `list_item_with_inline_styles` — list item emits styled spans.

### Layer 3 — Rendering
- [ ] `blockquote_renders_inline_styles` — TestBackend buffer shows styled text inside blockquote.

## Files touched

- `crates/runie-core/src/markdown/blocks.rs`
- `crates/runie-core/src/layout.rs`
- `crates/runie-tui/src/message/mod.rs`

## Notes

- This removes a deliberate limitation, not a `pulldown-cmark` limitation.
