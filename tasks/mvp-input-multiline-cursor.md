# Multi-line input cursor polish

**Status**: done
**Milestone**: MVP
**Category**: Input & Commands

## Description

Remaining polish for multi-line input: backspace at line start and cursor positioning.

## Acceptance Criteria

- [x] Backspace at line start joins with previous line
- [x] Cursor up/down moves between lines correctly
- [x] Home/End work per-line in multi-line input

## Tests

### Layer 1 — State/Logic
- [x] `backspace_at_line_start_joins_lines` — `backspace_at_line_start_removes_newline` in `input_grapheme.rs`
- [x] `cursor_up_moves_to_previous_line` — `move_cursor_up_navigates` in `line_nav.rs`
- [x] `cursor_down_moves_to_next_line` — `move_cursor_down_navigates` in `line_nav.rs`

### Layer 2 — Event Handling
- [x] `backspace_key_joins_lines` — Added in `input_grapheme.rs`

## Implementation

### Files
- `crates/runie-core/src/update/input.rs` — `delete_before_cursor()` handles newline removal
- `crates/runie-core/src/update/line_nav.rs` — `move_cursor_up()`, `move_cursor_down()`, `move_cursor_to_line_start()`, `move_cursor_to_line_end()`
- `crates/runie-core/src/update/mod.rs` — Event routing for HistoryPrev/HistoryNext → cursor navigation

### Key Behavior
1. **Backspace at line start**: `delete_before_cursor()` detects newline before cursor and removes it
2. **Cursor up/down**: `Event::HistoryPrev`/`Event::HistoryNext` routes to `move_cursor_up()`/`move_cursor_down()` when input has newlines
3. **Home/End per-line**: `cursor_start()`/`cursor_end()` call `move_cursor_to_line_start()`/`move_cursor_to_line_end()` when input has newlines

## Notes

- Deferred from `mvp-input-multiline` which covers core newline insertion
