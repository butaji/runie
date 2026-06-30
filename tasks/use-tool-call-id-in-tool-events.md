# Use tool-call id in ToolStart/ToolEnd events

**Status**: done
**Milestone**: R7
**Category**: Tools
**Priority**: P1

**Depends on**: fix-tui-mock-simple-text-response-repetition
**Blocks**: none

## Description

`ToolStart` and `ToolEnd` events are emitted with the request id (`cmd_id`) instead of the tool-call id. When a turn contains multiple tools, the events share the same id and the UI cannot distinguish or track individual tool calls.

## Root Cause

`crates/runie-agent/src/turn/tools.rs` uses `cmd_id.to_owned()` for event ids.

## Acceptance Criteria

- [ ] `ToolStart`/`ToolEnd` use `tool_call.id` when available.
- [ ] Falls back to `cmd_id` only when `tool_call.id` is missing.
- [ ] Multiple tools in one turn produce distinct event ids.
- [ ] `cargo test --workspace` passes.
- [ ] Live tmux multi-tool turn tracks each tool separately.

## Tests

### Layer 1 — State/Logic
- [ ] `tool_event_id_matches_tool_call_id` — emit two tools and assert distinct ids.

### Layer 2 — Event Handling
- [ ] `tool_end_with_matching_id_clears_tool` — `ToolEnd { id }` clears the tool matching that id.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A for mock; covered by unit/e2e tests.

## Files touched

- `crates/runie-agent/src/turn/tools.rs`
- `crates/runie-core/src/event.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- This is a correctness issue for multi-tool turns and UI tracking.
