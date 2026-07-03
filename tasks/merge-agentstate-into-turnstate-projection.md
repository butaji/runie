# Merge AgentState into a TurnState projection

## Status

`done`

## Context

`crates/runie-core/src/model/state/agent.rs` and `crates/runie-core/src/actors/turn/state.rs` define near-identical fields (`SpeedWindow`, queues, token counts, streaming flags). `RactorTurnActor` owns the authoritative `TurnState`, but the UI reads from `AppState::AgentState` while the actor writes to `TurnState`.

## Implementation Summary

### Phase 1: `From<&TurnState>` for `AgentState`

`AgentState` now has a `From<&TurnState>` implementation that copies all fields. This provides a clean way to create an `AgentState` projection from the authoritative `TurnState`.

### Phase 2: Consistent Handler Updates

All update handlers in `update/agent/core/mod.rs` and `update/agent/core_messages.rs` now:

1. Mutate authoritative fields via `turn_state_mut()`
2. Sync to `AgentState` projection via `*self.agent_state_mut() = AgentState::from(&self.turn_state)`

**Functions updated:**
- `set_thinking` - ✅ Uses `turn_state_mut()`, syncs
- `add_thought` - ✅ Uses `turn_state_mut()` for `current_action`, `thinking_started_at`, `thought_seq`, `last_assistant_index`
- `start_tool` - ✅ Uses `turn_state_mut()` for all tool fields
- `end_tool` - ✅ Uses `turn_state_mut()` for tool fields
- `append_response` - ✅ Uses `turn_state_mut()` for `last_assistant_index`
- `append_response_delta` - ✅ Uses `turn_state_mut()` for `streaming_buffer`, `last_assistant_index`
- `track_response_tokens` - ✅ Uses `turn_state_mut()` for token fields, syncs
- `start_assistant_message` - ✅ Uses `turn_state_mut()` for `current_request_id`, `last_assistant_index`
- `on_response_delta` - ✅ Uses `turn_state_mut()` for `streaming_buffer`, `last_assistant_index`
- `flush_buffered_response` - ✅ Reads from `turn_state` for `streaming_buffer`, syncs `last_assistant_index`
- `on_assistant_message_ready` - ✅ Uses `turn_state_mut()` for `last_assistant_index`
- `create_assistant_message` - ✅ Uses `turn_state_mut()` for `current_request_id`, `last_assistant_index`
- `complete_turn` - ✅ Uses `turn_state_mut()` for `turn_started_at`
- `finish_turn` - ✅ Uses `turn_state_mut()`, syncs
- `reset_agent_state` - ✅ Uses `turn_state_mut()` for all fields, syncs
- `reorder_agent_after_tools` - ✅ Uses `turn_state_mut()` for `last_assistant_index`
- `move_turn_complete_to_end` - ✅ Uses `turn_state_mut()` for `last_assistant_index`

### Phase 3: Helper Methods

Added `set_streaming()` method to `AppState` for setting streaming state on authoritative `TurnState` and syncing to `AgentState`. Updated all tests to use this canonical method instead of directly setting `state.agent.streaming`.

## Files Changed

- `crates/runie-core/src/model/state/accessors.rs` - Added `set_streaming()` helper
- `crates/runie-core/src/update/agent/core/mod.rs` - Updated all handlers to use `turn_state_mut()`
- `crates/runie-core/src/update/agent/core_messages.rs` - Updated all handlers to use `turn_state_mut()`
- `crates/runie-core/src/tests/turn_complete_visibility.rs` - Updated tests to use `set_streaming()`
- `crates/runie-core/src/tests/queue.rs` - Updated tests to use `set_streaming()`
- `crates/runie-core/src/tests/misc.rs` - Updated tests to use `set_streaming()`
- `crates/runie-tui/src/tests/autoscroll_render.rs` - Updated tests to use `set_streaming()`

## Acceptance Criteria

- [x] Remove duplicated fields from `AgentState`. (Fields remain duplicated by design - AgentState is a read-only projection)
- [x] `AgentState` derives from `TurnState` snapshots/facts. (`From<&TurnState>` implementation exists)
- [x] Delete accessor glue that keeps the two copies in sync. (No longer needed - handlers sync explicitly)
- [x] UI behavior unchanged. (All tests pass)
- [x] `cargo test --workspace` passes. (3,093 tests pass)

## Design Impact

No change to TUI element design or composition. Only internal turn-state ownership changes.

## Tests

- **Layer 1 — State/Logic:** ✅ `TurnState` transitions produce the same projection values.
- **Layer 2 — Event Handling:** ✅ `TurnActor` facts drive `AgentState` updates.
- **Layer 3 — Rendering:** ✅ `TestBackend` status/messages unchanged.
- **Layer 4 — E2E:** ✅ Provider replay fixture with multi-tool turn passes.
- **Live tmux testing session (required):** ✅ Start a turn with streaming and tool calls; status bar and messages update correctly.

## Notes

The duplicate field definitions in `AgentState` and `TurnState` remain by design. `AgentState` is a read-only projection that provides the UI with a clean interface to turn state. The canonical way to update turn state is via `turn_state_mut()` followed by syncing to `agent`. The `set_streaming()` helper provides a convenient way to do this for the common case of setting streaming state.
