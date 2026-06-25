# Extract shared streaming tool-call parser

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: canonicalize-tool-call
**Blocks**: dedupe-tool-execution-loop

## Description

`crates/runie-agent/src/stream_response.rs` and `crates/runie-agent/src/headless.rs` both define `ToolCallAccumulator { name, arguments }` and nearly identical state machines for handling `LLMEvent::TextDelta`, `ToolCallStart`, `ToolCallInputDelta`, `ToolCallEnd`, `Finish`, and `Error`. Divergence in tool-call ID handling, malformed JSON, or finish-state races must be fixed twice.

**Implemented**: Removed the duplicate `ToolCallAccumulator` from `headless.rs` and refactored it to use the shared `ToolStream` from `runie-core/src/tool_stream.rs`. The `stream_response.rs` already used `ToolStream`.

## Acceptance Criteria

- [x] A single `StreamingToolParser` / `StreamState` module exists in `runie-core` or `runie-agent`. — `ToolStream` exists in `runie-core/src/tool_stream.rs`
- [x] Both existing paths are rewritten to use the shared module. — `headless.rs` now uses `ToolStream`
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `streaming_tool_parser_accumulates_tool_call` — covered by existing `tool_stream::tests` in runie-core
- [x] `streaming_tool_parser_falls_back_to_inline_json` — covered by existing tests in `stream_response.rs`

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] Existing E2E tests cover the streaming parser paths.

## Files touched

- `crates/runie-agent/src/headless.rs` — removed local `ToolCallAccumulator`, uses shared `ToolStream`
- `crates/runie-core/src/tool_stream.rs` — the shared module (already existed)

## Notes

The `ToolStream` in `runie-core` already provided the shared accumulation logic. The task was to refactor `headless.rs` to use it instead of maintaining its own duplicate accumulator. The observer/callback parameterization was not needed since both paths use `ToolStream` identically.
