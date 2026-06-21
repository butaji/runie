# Unify interactive and headless tool execution loops

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: extract-streaming-tool-parser
**Blocks**: none

## Description

`crates/runie-agent/src/turn.rs` (`execute_tools`) and `crates/runie-agent/src/headless.rs` (`execute_headless_tools`) both iterate over parsed tool calls, build a `ToolContext`, get the `builtin_registry()`, call `execute_tool_call()`, and push a tool-result message. The interactive path additionally emits `AgentEvent::ToolStart`/`ToolEvent` events and increments a counter.

## Acceptance Criteria

- [ ] One shared tool-execution helper handles both paths.
- [ ] The helper accepts an optional observer callback for event emission.
- [ ] Permission gate, registry lookup, error fallback, and result-message formatting live in one place.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `execute_tool_call_builds_result_message` — shared helper produces correct `ChatMessage` for a tool result.

### Layer 2 — Event Handling
- [ ] `interactive_tool_execution_emits_events` — with an event observer, `ToolStart`/`ToolEnd` are emitted.
- [ ] `headless_tool_execution_silent` — without observer, no events are emitted.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `multi_tool_turn_headless_and_interactive_match` — same provider fixture run both ways produces the same final messages.

## Files touched

- `crates/runie-agent/src/turn.rs`
- `crates/runie-agent/src/headless.rs`
- `crates/runie-core/src/tool/mod.rs` or new `crates/runie-agent/src/tool_executor.rs`

## Notes

Do after `extract-streaming-tool-parser` so the shared tool-call accumulation can feed the same executor.
