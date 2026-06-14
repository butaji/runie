# Remove Manual Element Line-Count Sync Between Core and TUI

**Status**: done
**Milestone**: R3
**Category**: TUI Rendering
**Priority**: P1

## Description

`runie-core/src/ui/elements.rs::line_count` computes logical line counts for scroll
math, while `runie-tui/src/ui.rs::to_lines` computes actual wrapped Ratatui lines. The
core code contains an explicit comment: “Must stay in sync with `to_lines()` in
`runie-tui/src/ui.rs`.” This coupling is fragile.

## Acceptance Criteria

- [x] The line count for an element is derived from the same rendering logic that
  produces the actual lines, eliminating the need to keep two functions in sync.
- [x] Chosen approach: expose `runie_tui::ui::element_line_count` (delegates to the
  renderer's `to_lines`) and compute width-aware line counts in core via the shared
  `runie_core::layout` helper.
- [x] Scroll offset, scrollbar thumb, and vim-nav selection remain correct.
- [x] All existing feed/scroll/vim tests pass.

## Tests

### Layer 1 — State/Logic
- [x] `element_line_count_matches_rendered_lines` for user messages, agent messages,
  tool output, and simple variants.

### Layer 2 — Event Handling
- [x] `page_down_scrolls_by_rendered_lines`.

### Layer 3 — Rendering
- [x] `scrollbar_thumb_position_matches_line_count`.

## Files touched

- `crates/runie-core/src/ui/elements.rs`
- `crates/runie-core/src/ui/transform.rs`
- `crates/runie-core/src/snapshot.rs`
- `crates/runie-tui/src/ui.rs`
- `crates/runie-core/src/tests/scroll.rs`
- `crates/runie-core/src/tests/vim_element_jump.rs`

## Out of scope

- Rewriting the wrapping algorithm.
