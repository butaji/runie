# Use Display-Cell Width for Wrapping and Alignment

**Status**: done
**Milestone**: R3
**Category**: TUI Rendering
**Priority**: P1

## Description

Wrapping and width math in `runie_core::layout` and `runie_tui::message` used `chars().count()`, which is wrong for CJK and emoji (display width 2). This caused misalignment and overflow.

## Acceptance Criteria

- [x] Replace `chars().count()` with display-cell width in layout and message modules.
- [x] Timestamp alignment remains correct with wide characters.
- [x] Wrapping does not split wide characters.

## Tests

### Layer 1 — State/Logic
- [x] `wide_character_counts_as_two_cells`.
- [x] `wrap_respects_display_width`.

### Layer 3 — Rendering
- [x] `wide_text_does_not_overflow_viewport`.
