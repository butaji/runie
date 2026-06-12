# Refactor: Unify Tool Markers Parsing

**Status**: done
**Milestone**: R3
**Category**: Core Architecture

## Description

Tool markers are parsed in two places with fragile string matching:

1. `crates/runie-core/src/update/mod.rs` — `strip_tool_markers()` and `content_has_tool_markers()` use string operations
2. `crates/runie-agent/src/parser.rs` — structured `parse_tool_calls()` using serde_json

The core should use the agent's parser instead of duplicating the logic. This prevents drift and handles edge cases properly.

## Acceptance Criteria

- [x] `update/strip_tool_markers()` calls into agent parser or shares logic
- [x] `update/content_has_tool_markers()` uses shared parsing
- [x] Test with content containing "TOOL:" as legitimate text (not a marker)
- [x] Test with malformed tool calls that look like markers
- [x] All existing tests pass

## Tests

### Layer 1 — State/Logic
- [x] `test_strip_tool_markers_handles_legitimate_tooltip_text` — "Use the TOOL: parameter" should NOT be stripped
- [x] `test_strip_tool_markers_handles_valid_tool_call` — JSON with name/arguments is stripped
- [x] `test_strip_tool_markers_handles_multiple_tools` — Multiple tool calls all stripped
- [x] `test_has_tool_markers_positive` — Returns true for valid markers
- [x] `test_has_tool_markers_negative` — Returns false for normal text

### Layer 2 — Event Handling
- [ ] `test_agent_response_with_tool_markers` — verifies markers stripped from display

### Layer 3 — Rendering
N/A

### Layer 4 — Smoke
- [ ] `smoke_agent_tool_execution.sh` — real tool call, verify output

## Notes

Consider creating a shared `ToolMarker` module in `runie-core` that both `update` and `runie-agent` can use, or making `runie-agent::parser` public and using it from core.

**Out of scope**: Changing the tool call format or protocol
