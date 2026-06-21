# Canonicalize ToolCall type

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P0

**Depends on**: none
**Blocks**: extract-streaming-tool-parser

## Description

The same tool-invocation concept is represented by at least six types: `message::ToolCall`, `tool_runtime::ToolCall`, `tool_parser::ParsedToolCall`, `message::parts::Part::ToolCall`, `runie_provider::openai::stream::ToolCallDelta`, `runie_json::ToolCall`, and `runie_agent` accumulators. Conversions are scattered across agent, provider, and UI.

## Acceptance Criteria

- [ ] One canonical `ToolCall { id, name, args: Value }` exists in `runie-core`.
- [ ] All other occurrences are removed or become thin `From` adapters.
- [ ] Provider-specific streaming deltas accumulate into the canonical type.
- [ ] `cargo test --workspace` and `cargo check --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [ ] `canonical_tool_call_round_trips_through_json` — serialization/deserialization unchanged.
- [ ] `parsed_tool_call_maps_to_canonical` — parser output converts correctly.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `openai_stream_accumulates_canonical_tool_calls` — replay SSE fixture and assert final calls match the canonical type.

## Files touched

- `crates/runie-core/src/message/mod.rs`
- `crates/runie-core/src/tool_runtime.rs`
- `crates/runie-core/src/tool_parser/mod.rs`
- `crates/runie-core/src/message/parts.rs`
- `crates/runie-provider/src/openai/stream.rs`
- `crates/runie-json/src/main.rs`
- `crates/runie-agent/src/stream_response.rs`
- `crates/runie-agent/src/headless.rs`

## Notes

`Value` from `serde_json` is the natural argument representation. Provider deltas may keep a private streaming state object, but the accumulated result must be the canonical type.
