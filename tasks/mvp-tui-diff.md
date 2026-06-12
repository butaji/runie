# Diff rendering for edits

**Status**: done

**Milestone**: MVP

**Category**: TUI Rendering

## Description

Render diff output for file edits with unified diff format, added/removed line highlighting, and line numbers.

## Acceptance Criteria

- [x] Unified diff format (---, +++, @@ hunks)
- [x] Added/removed line highlighting (green/red)
- [x] Line numbers in gutter

## Implementation

### Files

- `crates/runie-agent/src/diff.rs` — Unified diff generation with LCS algorithm
- `crates/runie-tui/src/diff.rs` — Diff parsing and rendering with styling
- `crates/runie-agent/src/tools.rs` — Updated edit_file to produce diff output
- `crates/runie-tui/src/ui.rs` — Updated render_tool_done to detect and highlight diffs

### Architecture

1. **Diff generation** (`runie-agent`):
   - LCS-based diff algorithm produces hunks with added/removed/context lines
   - `render_diff_to_string()` outputs unified diff format

2. **Diff rendering** (`runie-tui`):
   - `is_diff_output()` detects diff format
   - `parse_diff()` parses unified diff into typed lines
   - `render_diff_text()` produces styled ratatui Lines
   - Added lines: green (C.success)
   - Removed lines: red
   - Context lines: default color
   - Line numbers in dim gutter

## Tests

### Layer 1 — State/Logic (runie-agent)
- [x] `no_changes_empty_diff` — Identical content produces empty diff
- [x] `single_line_addition` — Adding a line produces added diff line
- [x] `single_line_removal` — Removing a line produces removed diff line
- [x] `single_line_modification` — Changing a line produces remove+add
- [x] `empty_old_content` — Empty to content produces diff
- [x] `empty_new_content` — Content to empty produces diff
- [x] `multi_line_addition` — Multiple line additions
- [x] `render_diff_to_string_format` — Correct unified diff format

### Layer 1 — State/Logic (runie-tui)
- [x] `detects_diff_output` — Correctly identifies diff format
- [x] `rejects_non_diff_output` — Non-diff text not detected as diff
- [x] `parses_simple_diff` — Parses basic diff structure
- [x] `parses_hunk_header` — Extracts hunk header correctly
- [x] `diff_line_styles` — Correct styles for line types
- [x] `diff_line_prefixes` — Correct prefixes (+/-/space)
- [x] `empty_content` — Handles empty input
- [x] `preserves_line_numbers` — Line numbers correctly tracked

### Layer 3 — Rendering (runie-tui)
- [x] `render_diff_output` — Produces styled Line output
- [x] `render_non_diff_as_plain` — Non-diff rendered as plain text

All 18 diff tests pass.
