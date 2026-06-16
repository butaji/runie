# Tool Block Inline Status

**Status**: done
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P1
**Completed in**: current

**Depends on**: (none)
**Blocks**: (none)

## Description

All tool block status criteria were already implemented before this task. This
task adds a `tool_status_line` helper for testability, adds the Layer 1 and
Layer 3 tests from the task template, and verifies all acceptance criteria.

## Acceptance Criteria

- [x] Running tool blocks show spinner + label + elapsed time (`⠋ Run List . 1.8s`)
  — already rendered in `render_tool_running`; confirmed by existing test
  `render_tool_running_shows_duration` and new test `tool_status_line_running_shows_spinner`
- [x] Completed tool blocks show total duration + byte count + status
  (`5.7s ⇣21.2k [✓]`) — already rendered in `render_tool_done`
- [x] Failed tool blocks show `[✗]` and error count — `render_tool_done` adds ` [✗]`
- [x] Byte counts are humanized (`21.2k`, `4.93k`) — `format_bytes()` in `tool/mod.rs`

## Changes Made

### 1. `tool_status_line` helper (`tool/mod.rs`)
Added a pure formatting function matching the rendered header format:
`status glyph + label + duration + bytes + error_suffix`.
This enables fast unit tests without rendering infrastructure.

### 2. Layer 1 tests (`tool/mod.rs` tests)
- `tool_status_line_includes_duration` — status line contains duration
- `tool_status_line_formats_bytes` — `4930` bytes → `4.9k`
- `tool_status_line_running_shows_spinner` — starts with `⠋`, contains `Xs`
- `tool_status_line_done_shows_checkmark` — starts with `✓`, shows duration + bytes
- `tool_status_line_error_shows_error_icon` — starts with `✗`, ends with ` [✗]`

### 3. Layer 3 test (`tests/render/tools.rs`)
Added `render_tool_done_shows_duration` confirming the rendered output contains
the duration string (e.g. `5.7s`).

## Files touched

- `crates/runie-core/src/tool/mod.rs` (tool_status_line + 5 tests)
- `crates/runie-tui/src/tests/render/tools.rs` (render_tool_done_shows_duration)

## Existing implementation (no changes needed)

The actual rendering was already complete:
- `ToolRunning`, `ToolDone`, `ToolSummary` elements in `ui/elements.rs`
- `render_tool_running` in `message/support.rs` — `⠋ Run {name} {duration}s`
- `render_tool_done` in `message/support.rs` — `✓/✗ Run {name} {duration} ⇣{bytes}`
- `render_tool_summary` in `message/support.rs` — `✓ {name} {duration} [+]`
- `format_bytes` / `format_duration` / `format_tool_label` in `tool/mod.rs`
- Existing TUI Layer 3 tests in `tests/render/tools.rs`
