# Multi-line input cursor polish

**Status**: todo
**Milestone**: MVP
**Category**: Input & Commands

## Description

Remaining polish for multi-line input: backspace at line start and cursor positioning.

## Acceptance Criteria

- [ ] Backspace at line start joins with previous line
- [ ] Cursor up/down moves between lines correctly
- [ ] Home/End work per-line in multi-line input

## Tests

- [ ] Layer 1 — `backspace_at_line_start_joins_lines`
- [ ] Layer 1 — `cursor_up_moves_to_previous_line`
- [ ] Layer 1 — `cursor_down_moves_to_next_line`
- [ ] Layer 2 — `backspace_key_joins_lines`

## Notes

- Deferred from `mvp-input-multiline` which covers core newline insertion
