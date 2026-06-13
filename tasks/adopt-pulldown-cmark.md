# Adopt `pulldown-cmark` for Markdown Parsing

**Status**: todo
**Milestone**: R3
**Category**: TUI Rendering
**Priority**: P1

**Depends on**: crate-replacement-audit

## Description

Replace the hand-rolled markdown parser in `crates/runie-tui/src/markdown.rs`
with `pulldown-cmark` for parsing. The Ratatui rendering layer is kept
because tool-call interleaving is Runie-specific. Context7 ID:
`/pulldown-cmark/pulldown-cmark`.

## Acceptance Criteria

- [ ] Add `pulldown-cmark = "0.13"` to `crates/runie-tui/Cargo.toml`.
- [ ] Replace inline parsing (`parse_inline_markdown`) with `pulldown_cmark::Parser`.
- [ ] Replace block extraction (`extract_code_blocks`) with event-based block
  collection from `Parser`.
- [ ] Enable GFM extensions: tables, strikethrough, task lists.
- [ ] Existing `MdSpan` and `CodeBlock` types remain the public interface so
  callers do not change.
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `parse_bold_italic_code` — same spans as current parser.
- [ ] `extract_code_blocks_fenced` — code blocks extracted correctly.
- [ ] `parse_table` — GFM table parsed (new capability).

### Layer 3 — Rendering
- [ ] `markdown_block_renders_table` — table appears in TestBackend output.

## Notes

**ctx7 snippet:**
```rust
use pulldown_cmark::{Event, Parser, Tag, Options};
let mut options = Options::empty();
options.insert(Options::ENABLE_STRIKETHROUGH);
let parser = Parser::new_ext(markdown, options);
for event in parser { match event { Event::Text(t) => ... } }
```

**Files touched:**
- `crates/runie-tui/Cargo.toml`
- `crates/runie-tui/src/markdown.rs`

**Out of scope:**
- Full CommonMark edge cases that Ratatui cannot render (e.g., embedded HTML).
- Replacing the renderer with `ratatui-markdown`.
