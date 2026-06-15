# Unify Markdown-Aware Line Counts Between Core and TUI

**Status**: done
**Milestone**: R3
**Category**: TUI / Rendering
**Priority**: P1

## Description

`runie_core::layout::element_line_count` treated agent messages as plain text, while `runie_tui::message::render_agent_message` parsed them into code blocks, lists, and blockquotes. This caused scroll/vim-nav/scrollbar errors for markdown-rich messages.

## Acceptance Criteria

- [x] Core line counting uses the same markdown-block decomposition as the renderer.
- [x] `Snapshot`/`ViewState` scroll math is correct for code blocks, lists, and blockquotes.
- [x] Existing scrollbar/vim tests still pass.
- [x] New test compares core line count to actual `TestBackend` rendered rows.

## Tests

### Layer 1 — State/Logic
- [x] `agent_message_with_code_block_line_count_matches_rendered_rows`.
- [x] `agent_message_with_list_line_count_matches_rendered_rows`.

### Layer 3 — Rendering
- [x] `scrollbar_thumb_matches_markdown_message_height`.
