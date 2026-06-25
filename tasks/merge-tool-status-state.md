# Merge ToolCallState and ToolStatus

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`runie-core/src/tool/state.rs` defines `ToolCallState { Pending, Running, Completed, Error }` and `runie-core/src/tool/context.rs` defines `ToolStatus { Success, Error, TimedOut, Blocked, AwaitingUser }`. Both describe tool execution state with overlapping variants (`Completed/Success`, `Error/Error`).

## Acceptance Criteria

- [ ] The two enums are merged into one, or a clear one-way conversion is defined and documented.
- [ ] If kept separate, `ToolStatus` is the outcome and `ToolCallState` is the UI state machine; conversion is explicit.
- [ ] All callers use the canonical enum.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `tool_state_maps_to_status` — every `ToolCallState` variant maps to the correct `ToolStatus`.
- [ ] `tool_status_round_trips` — merged enum behaves as before.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tool_status_in_headless_turn` — headless turn reports the same status as before.

## Files touched

- `crates/runie-core/src/tool/state.rs`
- `crates/runie-core/src/tool/context.rs`
- All callers.

## Notes

A merged enum is preferred unless there is a genuine semantic split (e.g., UI state needs `Pending` while outcome does not).
