# Unify Markdown Pipeline

**Status**: done
**Milestone**: R3
**Category**: TUI / Rendering
**Priority**: P2

**Depends on**: unify-rendering-pipeline
**Blocks**: (none)

**Completed**: 2026-06-16

## Description

Markdown is parsed twice:

- `runie-core/src/markdown.rs` decomposes blocks for scroll math.
- `runie-tui/src/markdown.rs` re-parses inline spans for styling.

Both use `pulldown-cmark`. Keeping them in sync is manual work, and the
line counts computed in core can drift from the rendered output in TUI.

## Acceptance Criteria

- [x] A single markdown pass produces a styled AST usable by both core and
  TUI.
- [x] `runie-core` uses the AST for scroll/line-count math.
- [x] `runie-tui` renders the AST directly without re-parsing.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `ast_line_count_matches_render` — the AST line count equals the
  rendered line count for sample messages.

### Layer 3 — Rendering
- [x] `styled_spans_preserved` — bold/italic/code spans survive the unified
  pipeline.

## Files touched

- `crates/runie-core/src/markdown.rs`
- `crates/runie-core/src/layout.rs`
- `crates/runie-tui/src/markdown.rs`
- `crates/runie-tui/src/message/mod.rs`
- `crates/runie-tui/src/message/wrap.rs`
- `crates/runie-core/build.rs`

## Notes

- Added `MdInline::is_break()` so core can reconstruct plain text from the
  AST for wrapping/line-count math.
- Core `layout::markdown_block_line_count` now uses `CodeBlock::Text.inlines`
  instead of the raw `content` string.
- TUI `message::render_agent_text_block` renders the pre-parsed `inlines`
  via `apply_color_to_inlines` and a new style-preserving wrapper.
- TUI user messages also avoid per-chunk re-parsing by wrapping styled spans.
- Removed obsolete per-chunk re-parsing helpers (`build_agent_line`,
  `render_msg_line`, `msg_line_widths`, `msg_chunk_line`).
