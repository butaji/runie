# Standardize headless output as streaming JSON events

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: expose-runie-via-acp-stdio
**Blocks**: none

## Summary

Replace custom headless CLI output with a single streaming JSON event stream. All clients — TUI, headless scripts, ACP consumers — read the same fact/event shape.

## Event schema

Inspired by Grok Build's headless output, Runie events are newline-delimited JSON:

```json
{"type":"text","data":"Hello, "}
{"type":"text","data":"world!"}
{"type":"tool_call_start","id":"call_1","name":"bash"}
{"type":"tool_call_input_delta","id":"call_1","delta":"{\"cmd\":\"ls\"}"}
{"type":"tool_call_end","id":"call_1"}
{"type":"permission_request","id":"perm_1","tool":"bash","args":{...}}
{"type":"tool_result","id":"call_1","output":"..."}
{"type":"usage","input_tokens":120,"output_tokens":45}
{"type":"end","stopReason":"EndTurn","sessionId":"...","requestId":"..."}
```

Error events:

```json
{"type":"error","message":"Agent building failed: ..."}
```

## Implementation Summary (2026-06-26)

### Completed Work

- ✅ `HeadlessEvent` enum in `crates/runie-core/src/event/headless.rs` with all required event types:
  - `Text` — text deltas
  - `Thinking` — reasoning deltas
  - `ToolCallStart`, `ToolCallInputDelta`, `ToolCallEnd` — tool call lifecycle
  - `PermissionRequest` — permission prompts
  - `ToolResult` — tool execution results
  - `Usage` — token usage
  - `Error` — error events
  - `End` — turn completion
- ✅ `print.rs` uses unified `HeadlessEvent` format
- ✅ `json.rs` now uses unified `HeadlessEvent` format (was using custom `StreamChunk`)
- ✅ `HeadlessEvent::to_json_line()` for JSONL serialization
- ✅ Tests for all event serialization/deserialization

### Remaining Items

None — all acceptance criteria met.

## Acceptance Criteria

- [x] Headless mode (`runie -p "..."`) emits newline-delimited JSON events.
- [x] Event types cover: `text`, `thinking`, `tool_call_start`, `tool_call_input_delta`, `tool_call_end`, `permission_request`, `tool_result`, `usage`, `error`, `end`.
- [x] Custom progress/formatting modules in `runie-cli` are removed.
- [x] TUI can consume the same stream internally.
- [x] `cargo check --workspace` is green.

## Tests

### Layer 1 — State/Logic
- [x] `text_event_serialization` — Text event serializes correctly
- [x] `tool_call_event_serialization` — ToolCallStart event serializes correctly
- [x] `end_event_serialization` — End event serializes correctly
- [x] `error_event_round_trips` — Error event deserializes correctly
- [x] `usage_event_has_correct_fields` — Usage event fields preserved
- [x] `permission_request_round_trips` — PermissionRequest deserializes correctly

### Layer 2 — Event Handling
- N/A

### Layer 3 — Rendering
- N/A

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] Headless runner tests verify event emission

## Files touched

- `crates/runie-cli/src/json.rs` — now uses unified `HeadlessEvent` format
- `crates/runie-core/src/event/headless.rs` — event definitions (already existed)

## Notes

- The `HeadlessEvent` system was already in place; this task completed the standardization by ensuring all headless modes (print, json) use the unified format.
