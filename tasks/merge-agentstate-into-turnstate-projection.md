# Merge AgentState into a TurnState projection

## Status

`partial`

## Context

`crates/runie-core/src/model/state/agent.rs:7-143` and `crates/runie-core/src/actors/turn/state.rs:15-114` define near-identical fields (`SpeedWindow`, queues, token counts, streaming flags). `RactorTurnActor` owns the authoritative `TurnState`, but the UI reads from `AppState::AgentState` while the actor writes to `TurnState`.

## Progress Made

1. **`From<&TurnState>` for `AgentState`** - `AgentState` now has a `From<&TurnState>` implementation that copies all fields.

2. **Sync helper methods** - Added `set_turn_active()` and `sync_agent_state()` to `AppState` for tests.

3. **Update handlers use `turn_state_mut()`** - Many update handlers now mutate `turn_state_mut()` and sync to `agent`.

## Remaining Work

### Files/Functions Still Directly Mutating `agent_state_mut()`

1. **`core/mod.rs`:**
   - `add_thought` - sets `last_assistant_index`, `thought_seq`
   - `start_tool` - sets `current_request_id`, `current_tool_name`, etc.
   - `end_tool` - sets various fields
   - `append_response` - sets `last_assistant_index`
   - `append_response_delta` - sets `streaming_buffer`, `last_assistant_index`
   - `complete_turn` - sets `turn_started_at`
   - `add_error` - calls `reset_agent_state`

2. **`core_messages.rs`:**
   - `close_open_parts` - reads from `agent`
   - `reorder_agent_after_tools` - reads/sets `last_assistant_index`
   - `move_turn_complete_to_end` - reads/sets `last_assistant_index`

### Root Cause

The functions in `core/mod.rs` directly mutate `agent_state_mut()` instead of going through `turn_state_mut()`. When these functions are called, they modify `agent` but not `turn_state`. Then when another function syncs from `turn_state` to `agent` (like `clear_turn_state`), the changes are lost.

## Acceptance Criteria

- [ ] Remove duplicated fields from `AgentState`.
- [ ] `AgentState` derives from `TurnState` snapshots/facts.
- [ ] Delete accessor glue that keeps the two copies in sync.
- [ ] UI behavior unchanged.
- [ ] `cargo test --workspace` passes.

## Design Impact

No change to TUI element design or composition. Only internal turn-state ownership changes.

## Tests

- **Layer 1 — State/Logic:** `TurnState` transitions produce the same projection values.
- **Layer 2 — Event Handling:** `TurnActor` facts drive `AgentState` updates.
- **Layer 3 — Rendering:** `TestBackend` status/messages unchanged.
- **Layer 4 — E2E:** Provider replay fixture with multi-tool turn passes.
- **Live tmux testing session (required):** Start a turn with streaming and tool calls; status bar and messages update correctly.
