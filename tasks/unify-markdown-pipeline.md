# Unify Markdown Pipeline

**Status**: todo
**Milestone**: R3
**Category**: TUI / Rendering
**Priority**: P2

**Depends on**: unify-rendering-pipeline
**Blocks**: (none)

## Description

Markdown is parsed twice:

- `runie-core/src/markdown.rs` decomposes blocks for scroll math.
- `runie-tui/src/markdown.rs` re-parses inline spans for styling.

Both use `pulldown-cmark`. Keeping them in sync is manual work, and the
line counts computed in core can drift from the rendered output in TUI.

## Acceptance Criteria

- [ ] A single markdown pass produces a styled AST usable by both core and
  TUI.
- [ ] `runie-core` uses the AST for scroll/line-count math.
- [ ] `runie-tui` renders the AST directly without re-parsing.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `ast_line_count_matches_render` — the AST line count equals the
  rendered line count for sample messages.

### Layer 3 — Rendering
- [ ] `styled_spans_preserved` — bold/italic/code spans survive the unified
  pipeline.

## Files touched

- `crates/runie-core/src/markdown.rs`
- `crates/runie-tui/src/markdown.rs`
- `crates/runie-core/src/ui/transform.rs`
- `crates/runie-tui/src/message/mod.rs`

## Notes

This is a natural follow-up to `unify-rendering-pipeline`. Consider it
blocked until that task is done.
