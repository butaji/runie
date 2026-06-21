# Extract shared streaming tool-call parser

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: canonicalize-tool-call
**Blocks**: dedupe-tool-execution-loop

## Description

`crates/runie-agent/src/stream_response.rs` and `crates/runie-agent/src/headless.rs` both define `ToolCallAccumulator { name, arguments }` and nearly identical state machines for handling `LLMEvent::TextDelta`, `ToolCallStart`, `ToolCallInputDelta`, `ToolCallEnd`, `Finish`, and `Error`. Divergence in tool-call ID handling, malformed JSON, or finish-state races must be fixed twice.

## Acceptance Criteria

- [ ] A single `StreamingToolParser` / `StreamState` module exists in `runie-core` or `runie-agent`.
- [ ] It is parameterized by an observer trait or callback enum so `stream_response.rs` emits UI events and `headless.rs` appends to content/chunks.
- [ ] Both existing paths are rewritten to use the shared module.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `streaming_tool_parser_accumulates_tool_call` — feed deltas and assert final `ToolCall` is correct.
- [ ] `streaming_tool_parser_falls_back_to_inline_json` — when no structured events are emitted, inline JSON is parsed.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `interactive_turn_uses_shared_parser` — replay a provider fixture through the interactive path.
- [ ] `headless_turn_uses_shared_parser` — replay the same fixture through the headless path and assert identical tool calls.

## Files touched

- `crates/runie-agent/src/stream_response.rs`
- `crates/runie-agent/src/headless.rs`
- New `crates/runie-agent/src/stream_state.rs` or `crates/runie-core/src/streaming_tool_parser.rs`

## Notes

The shared parser should be independent of Ratatui/UI; it belongs in `runie-core` if it has no agent-specific dependencies.
