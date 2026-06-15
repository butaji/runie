# Fix Vim-Nav Row Mapping After Ratatui Wrap

**Status**: done
**Milestone**: R3
**Category**: TUI / Rendering
**Priority**: P1

## Description

`runie_tui::ui::messages::render_paragraph` hands pre-wrapped `Line`s to Ratatui but also sets `Wrap { trim: false }`. Long code lines are wrapped a second time by Ratatui, but `row_to_element` records only one mapping entry per pre-wrap line. This misaligns selection brackets and vim-nav row mapping.

## Acceptance Criteria

- [x] Either pre-wrap lines to the exact available width before handing to Ratatui, or compute `row_to_element` after Ratatui’s wrap pass.
- [x] Vim-nav selection covers all visible rows of an element.
- [x] Long code lines are handled correctly.

## Tests

### Layer 3 — Rendering
- [x] `vim_nav_selects_all_rows_of_wrapped_code_block`.
- [x] `row_to_element_len_equals_visible_rows`.
