# Multi-line input

**Status**: in-progress (was blocked - Shift+Enter and Ctrl+J handlers implemented)

**Milestone**: MVP

**Category**: Input & Commands

## Description

Support multi-line input editing.

## Acceptance Criteria

- [x] Shift+Enter for newlines
- [x] Ctrl+J for newlines
- [ ] Cursor positioning across lines
- [ ] Backspace at line start

## Implementation

### Files Modified
- `crates/runie-core/src/event.rs` — Added `Event::Newline`
- `crates/runie-core/src/update/mod.rs` — Added `Event::Newline` handling
- `crates/runie-core/src/update/input.rs` — Added `insert_newline()` method
- `crates/runie-term/src/main.rs` — Added Shift+Enter and Ctrl+J key mappings

### Key Bindings
- **Shift+Enter**: Insert newline
- **Ctrl+J**: Insert newline (line feed)

## Tests

### Layer 1 — State/Logic
- [x] `insert_newline_at_end` — Newline appended at cursor position
- [x] `insert_newline_in_middle` — Newline inserted at cursor position
- [x] `multiline_input_supported` — Multiple lines can be typed

### Layer 2 — Event Handling
- [x] Shift+Enter key mapping (in runie-term tests)
- [x] Ctrl+J key mapping (in runie-term tests)

### Layer 3 — Rendering
N/A (text editing, no TUI rendering)

### Layer 4 — Smoke
N/A (simple input handling)

## Remaining Work

- Cursor positioning across lines (already works with existing cursor movement)
- Backspace at line start (needs special handling to join lines)
