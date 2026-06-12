# Multi-line input

**Status**: done

**Milestone**: MVP

**Category**: Input & Commands

## Description

Support multi-line input editing.

## Acceptance Criteria

- [x] Shift+Enter for newlines
- [x] Ctrl+J for newlines
- [x] Cursor positioning across lines (works with existing cursor movement)
- [x] Backspace at line start (removes newline and joins lines)

## Implementation

### Files Modified
- `crates/runie-core/src/event.rs` — Added `Event::Newline`
- `crates/runie-core/src/update/mod.rs` — Added `Event::Newline` handling
- `crates/runie-core/src/update/input.rs` — Added `insert_newline()` and updated `delete_before_cursor()` for multi-line support
- `crates/runie-term/src/main.rs` — Added Shift+Enter and Ctrl+J key mappings

### Key Bindings
- **Shift+Enter**: Insert newline
- **Ctrl+J**: Insert newline (line feed)
- **Backspace at line start**: Join current line with previous line by removing newline

## Tests

### Layer 1 — State/Logic
- [x] `insert_newline_at_end` — Newline appended at cursor position
- [x] `insert_newline_in_middle` — Newline inserted at cursor position
- [x] `multiline_input_supported` — Multiple lines can be typed
- [x] `backspace_at_line_start_removes_newline` — Backspace removes newline at cursor position
- [x] `backspace_at_first_line_start_flashes` — Flash on backspace at absolute start
- [x] `backspace_removes_only_first_newline` — Correctly handles multiple newlines

### Layer 2 — Event Handling
- [x] Shift+Enter key mapping (in runie-term tests)
- [x] Ctrl+J key mapping (in runie-term tests)

### Layer 3 — Rendering
N/A (text editing, no TUI rendering)

### Layer 4 — Smoke
N/A (simple input handling)
