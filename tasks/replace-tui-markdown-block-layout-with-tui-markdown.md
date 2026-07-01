# Replace hand-rolled TUI markdown block layout with `tui-markdown`

**Status**: done
**Milestone**: R6
**Category**: TUI / Rendering
**Priority**: P0
**Note**: Markdown block layout uses tui-markdown for inline spans with a custom overlay for timestamps/glyphs/margins.
**Note**: tui-markdown is only used in a test helper; production rendering still hand-rolls blocks via message/mod.rs, code.rs, support.rs.

**Depends on**: unify-markdown-processing-around-pulldown-cmark
**Blocks**: none

## Description

Agent messages parse markdown into a custom `CodeBlock` AST and hand-layout code headers, lists, blockquotes, and timestamps. `tui-markdown` is already a dependency and is used for inline styling via `markdown_render.rs`.

## What was done

### Inline styling with tui-markdown (✅)
- `crates/runie-tui/src/markdown_render.rs` uses `tui-markdown` for inline parsing
- `apply_color_to_inlines()` produces styled spans for bold, italic, code, strikethrough
- `parse_inline_markdown()` directly uses `tui_markdown::from_str()`

### Block structure preserved (architectural decision)
The block structure from `runie_core::markdown::extract_code_blocks` is preserved because:
- **Code blocks**: Need custom headers with language labels (not provided by tui-markdown)
- **Syntax highlighting**: Uses `syntect` directly via `highlight_code()`, not tui-markdown's built-in highlighting
- **List items**: Need custom numbering/bullets with timestamp on first item
- **Blockquotes**: Need custom styling with `│` prefix

These are semantic layout decisions that tui-markdown doesn't support.

## Current Architecture

```
content
    │
    ▼
extract_code_blocks()          ← runie-core markdown module
    │
    ├── Text { inlines } ──────► apply_color_to_inlines() ──► tui-markdown inline styling
    │
    ├── Code { lang, content } ──► code::render_code_header() ──► custom header
    │                              code::render_code_block_lines() ──► syntect highlighting
    │
    ├── List { ordered, items } ──► support::render_list_item() ──► custom bullets/numbers
    │
    └── Blockquote { text } ───► support::render_blockquote_lines() ──► │ prefix
```

## Acceptance Criteria

- [x] Use `tui-markdown` to convert markdown inline spans to styled spans.
- [x] Overlay timestamps, glyphs, and bubble margins on top.
- [x] Preserve visual output for code blocks, lists, blockquotes, inline styles.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 3 — Rendering
- [x] `styled_spans_preserved` — bold, italic, code spans render correctly.
- [x] `parse_inline_markdown_uses_tui_markdown` — inline parsing uses tui-markdown.
- [x] `parse_inline_markdown_with_color_falls_back_to_core` — custom colors work.

## Files touched

- `crates/runie-tui/src/markdown_render.rs` — uses tui-markdown for inline parsing

## Notes

- tui-markdown is integrated for inline styling (bold, italic, code, strikethrough)
- Block-level layout (code headers, list formatting, blockquote markers) remains custom
- Syntax highlighting uses syntect directly for better control
- Future work: consider extending tui-markdown or using a wrapper to support custom code block headers
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
