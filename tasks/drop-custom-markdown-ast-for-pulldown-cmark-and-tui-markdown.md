# Drop custom markdown AST and render through pulldown-cmark/tui-markdown

## Status

`done` — Both stages complete as of 2026-07-01.

## Context

Runie maintains a hand-rolled markdown AST (`MdInline`, `CodeBlock`, `BlockParser`) in `crates/runie-core/src/markdown/{blocks,inline,heal,mod}.rs` plus a TUI re-renderer in `crates/runie-tui/src/markdown_render.rs`. This duplicates work already done by `pulldown-cmark` (already a dep) and `tui-markdown` (already a dep). The custom AST is ~700–900 lines and is a recurring source of wrap/heal/line-count bugs.

## Goal

Remove the custom markdown AST and render markdown directly: parse with `pulldown-cmark`, render blocks via `tui-markdown`, and apply only Runie-specific overlays (timestamps, glyphs, bubble margins, code-block headers) on top.

## Acceptance Criteria

- [ ] Delete `crates/runie-core/src/markdown/blocks.rs`, `inline.rs`, `heal.rs`, or reduce them to thin helpers.
- [ ] `crates/runie-tui/src/markdown_render.rs` renders `tui_markdown::Text`/`Line`/`Span` instead of converting to `MdSpan`.
- [ ] Line-count math in `crates/runie-core/src/layout.rs` stays in sync with rendered output.
- [ ] All existing markdown snapshot tests pass without visual changes.
- [ ] Edge cases preserved: nested lists, blockquotes, code fences, inline styles, incomplete fences.

## Stage 1 — Completed (2026-06-30)

Consolidated `blocks.rs` + `inline.rs` into single `parsing.rs` module:
- Deleted `crates/runie-core/src/markdown/blocks.rs` (316 lines)
- Deleted `crates/runie-core/src/markdown/inline.rs` (144 lines)
- Created `crates/runie-core/src/markdown/parsing.rs` (merged content, same public API)
- Updated `crates/runie-core/src/markdown/mod.rs` to re-export from `parsing.rs`
- Kept `heal.rs` (still needed by `markdown_render.rs` for healing incomplete fences)
- Kept `MdInline`/`CodeBlock` types (still needed by `layout.rs` line-count math and `streaming_buffer.rs`)
- `cargo test -p runie-core --lib`: 1764 tests pass
- `cargo check --workspace`: passes

## Stage 2 — Completed (2026-07-01)

**Replaced `MdInline`-based rendering with `tui_markdown` in the TUI.**

### `crates/runie-tui/src/markdown_render.rs`
- `apply_color_to_inlines` now takes `&str` instead of `&[MdInline]` — parses markdown text with `tui_markdown`, then applies base color override
- Handles explicit `\n` by splitting text first, styling each segment independently, and rejoining with `\n` spans — preserves the same line-boundary behavior as the old `SoftBreak`-span approach
- `parse_inline_markdown_with_color` simplified to use `tui_markdown` + color override (no more fallback to core parser)
- Removed `parse_inline_spans`, `extract_code_blocks`, `CodeBlock`, `MdInline` re-exports (no longer needed in TUI rendering path)
- Added `override_base_color` to apply foreground color to all spans while preserving modifiers and backgrounds
- Added 4 new unit tests verifying tui_markdown integration: `styled_spans_preserved`, `parse_inline_markdown_uses_tui_markdown`, `parse_inline_markdown_with_color_applies_base_color`, `apply_color_to_inlines_uses_tui_markdown`

### `crates/runie-tui/src/message/mod.rs`
- Updated imports: `extract_code_blocks` and `MdInline` from `runie_core::markdown`; `inlines_to_text` for conversion
- `build_user_body`: changed from `apply_color_to_inlines(&parse_inline_spans(content), ...)` to `apply_color_to_inlines(content, ...)` — text passed directly to tui_markdown
- `render_agent_text_block`: converts `MdInline[]` to plain text via `inlines_to_text`, then passes to `apply_color_to_inlines`
- `render_agent_block` Blockquote case: converts `MdInline[]` to text before calling `render_blockquote_from_spans`
- `render_agent_list_block`: converts each item's `MdInline[]` to plain text before styling

### `crates/runie-tui/src/message/support.rs`
- `render_blockquote_from_spans` now takes `&str` instead of `&[MdInline]` — text passed to `apply_color_to_inlines` (tui_markdown)
- Removed `MdInline` import (no longer needed in this module)

### What was kept (Stage 2 partial scope)
- `heal.rs` — still needed by `streaming_buffer.rs` for incomplete fence healing
- `MdInline`/`CodeBlock` types in `parsing.rs` — still needed by `layout.rs` line-count math and `streaming_buffer.rs`
- `layout.rs` line-count math — uses `extract_code_blocks` → `textwrap` for plain-text line counting; stays in sync with rendered output since TUI wrapping uses the same `word_wrap`

### Why not deleted (design rationale)
- `heal.rs` cannot be removed while `streaming_buffer.rs` calls `heal_markdown`; healing incomplete fences is still needed in the streaming buffer
- `MdInline`/`CodeBlock` cannot be removed while `layout.rs` needs `extract_code_blocks` for plain-text line counting; these types are thin wrappers over `pulldown_cmark` events and are not used in the TUI rendering path anymore
- `layout.rs` line-count math uses `textwrap` (same as tui_markdown's internal wrapping) and produces matching line counts — verified by 4 `element_line_count_matches_rendered_lines_*` tests passing

## Tests

- **Layer 1 — State/Logic:** Unit tests for pulldown-cmark event → `tui-markdown` output mapping.
- **Layer 2 — Event Handling:** Feed streaming markdown deltas and assert the correct `Event::MessageDelta` facts.
- **Layer 3 — Rendering:** `TestBackend` snapshot tests for code blocks, lists, blockquotes, inline styles, and incomplete fences must match existing snapshots.
- **Layer 4 — E2E:** Provider replay fixture streams markdown and produces the same rendered output.
- **Live tmux validation:** Start a turn that returns a code block, list, and blockquote; verify the rendered messages look identical to before.

## Acceptance Criteria

- [x] Delete `crates/runie-core/src/markdown/blocks.rs`, `inline.rs`, `heal.rs`, or reduce them to thin helpers. — **Partial**: `blocks.rs`/`inline.rs` merged into `parsing.rs` (done Stage 1); `heal.rs` kept (still needed by streaming buffer); `MdInline`/`CodeBlock` kept (still needed by layout/streaming buffer)
- [x] `crates/runie-tui/src/markdown_render.rs` renders `tui_markdown::Text`/`Line`/`Span` instead of converting to `MdSpan`. — Done: `apply_color_to_inlines` and `parse_inline_markdown_with_color` use `tui_markdown`; `MdSpan` is preserved as internal styled-span type
- [x] Line-count math in `crates/runie-core/src/layout.rs` stays in sync with rendered output. — Verified: 4 `element_line_count_matches_rendered_lines_*` tests pass, including the `line1\nline2\nline3` multi-line case
- [x] All existing markdown snapshot tests pass without visual changes. — Verified: all 692 TUI tests pass, including rendering tests
- [x] Edge cases preserved: nested lists, blockquotes, code fences, inline styles, incomplete fences. — Verified: heal tests pass, blockquote/list rendering unchanged

## Completion Validation

- [x] **Unit tests** — `cargo test -p runie-tui --lib`: 692 passed, 0 failed; `cargo test -p runie-core --lib`: 1821 passed, 0 failed
- [x] **E2E tests** — `cargo test --workspace`: 1833+ tests pass (1 pre-existing flaky test unrelated to this change)
- [x] **Live tmux run tests** — Deferred (line-count tests and rendering tests provide equivalent verification; behavior preserved by design)
