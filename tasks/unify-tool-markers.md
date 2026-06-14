# Unify Tool-Marker Parsing

**Status**: done
**Milestone**: R3
**Category**: Tools
**Priority**: P1

## Description

Tool-call marker parsing is duplicated:

- `crates/runie-core/src/tool_markers.rs` (181 LOC) — `has_tool_markers`,
  `parse_tool_calls`, `strip_tool_markers`.
- `crates/runie-agent/src/parser.rs` (130 LOC) — parses the same `TOOL:` and JSON
  formats into typed `Tool` values.

`docs/CRATE_DECISIONS.md` plans to retire `parser.rs` once providers emit structured
`LLMEvent`s, but until then there should be only one text parser.

## Acceptance Criteria

- [x] `runie-core/src/tool_markers.rs` is the single source of truth for detecting and
  stripping tool markers.
- [x] `runie-agent` delegates `has_tool_calls` to `runie_core::tool_markers::has_tool_markers`;
  typed `Tool` construction remains in `parser.rs` until `LLMEvent` is adopted.
- [x] All existing `parser.rs` and `tool_markers.rs` tests still pass.
- [x] No behavioral regressions in print/json/server tool execution.

## Tests

### Layer 1 — State/Logic
- [x] `parse_tool_calls_legacy_and_json` returns the same tool names from both parsers.
- [x] `strip_tool_markers_removes_only_tool_lines`.

### Layer 2 — Event Handling
- [ ] No event changes.

### Layer 3 — Rendering
- [ ] No rendering changes.

## Files touched

- `crates/runie-core/src/tool_markers.rs`
- `crates/runie-agent/src/parser.rs`
- `crates/runie-agent/src/turn.rs`
- `crates/runie-print/src/main.rs`
- `crates/runie-json/src/main.rs`

## Out of scope

- Full `LLMEvent` migration (covered by `tasks/llm-event-normalization.md`).
