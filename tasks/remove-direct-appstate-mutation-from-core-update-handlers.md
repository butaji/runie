# Remove direct `AppState` mutation from core update handlers

## Status

`done`

## Description

`AppState` is a read-only UI projection of actor-owned state. Several core update handlers still mutate `AgentState`, session messages, and turn state directly instead of routing changes through the owning actor and reacting to emitted events.

## Progress Made

### Core Changes (Completed)

1. **`apply_turn_aborted` in `system.rs`** - Now uses `turn_state_mut()` and syncs with `From<&TurnState>` instead of manual field-by-field sync.

2. **`apply_queue_delivery_sync` in `session.rs`** - Now mutates `turn_state_mut().message_queue` and `turn_state_mut().request_queue` instead of `agent_state_mut()`.

3. **`clear_turn_state` in `core_messages.rs`** - Now mutates `turn_state_mut()` directly instead of `agent_state_mut()`.

4. **`reset_agent_state` in `core_messages.rs`** - Now mutates `turn_state_mut()` and syncs to `agent`.

5. **`maybe_end_streaming` in `core_messages.rs`** - Now reads from `turn_state` and syncs to `agent`.

6. **`queue_steering_message` in `submit.rs`** - Now mutates `turn_state_mut()` and syncs to `agent`.

7. **`submit_user_message` in `submit.rs`** - Now uses `self.next_id()` which mutates `turn_state` and syncs.

8. **`apply_user_message_sync` in `submit.rs`** - Now mutates `turn_state_mut()` and syncs to `agent`.

9. **`estimate_and_add_tokens` in `submit.rs`** - Now mutates `turn_state_mut()` and syncs to `agent`.

10. **`queue_follow_up` in `session.rs`** - Now mutates `turn_state_mut()` and syncs to `agent`.

11. **`abort_queue` in `session.rs`** - Now mutates `turn_state_mut()` and syncs to `agent`.

12. **`dequeue` in `session.rs`** - Now pops from `turn_state_mut()` and syncs to `agent`.

13. **`set_thinking` in `core/mod.rs`** - Now mutates `turn_state_mut()` and syncs to `agent`.

14. **`next_id` in `domain_ops.rs`** - Now increments `turn_state_mut().next_id` and syncs to `agent`.

15. **`queue_steering_and_update_history` in `app_state.rs`** - Now mutates `turn_state_mut()` and syncs to `agent`.

16. **`AgentState` import** - Added to all files that need it.

17. **Helper methods** - Added `set_turn_active()` and `sync_agent_state()` to `AppState` for test use.

### Test Updates (Completed)

Updated tests to use `turn_state` fields instead of `agent` fields:
- `queue.rs`
- `flow.rs`
- `rapid_submit.rs`
- `vim_mode.rs`
- `counters.rs`
- `autoscroll.rs`
- `agent_error.rs`
- `input/tests.rs`

### Fixes Applied

1. **`track_response_tokens` in `core/mod.rs`** - Now updates `turn_state.tokens_out` and `turn_state.turn_tokens_out` directly, then syncs only those fields to `agent` (not the full `AgentState::from(&TurnState)` which would overwrite other fields like `last_assistant_index`).

2. **`add_thought` in `core/mod.rs`** - Now clears `turn_state.thinking_started_at` directly without doing a full sync (which would overwrite `last_assistant_index`).

3. **Test `test_complete_agent_flow`** - Fixed to pop from `turn_state.request_queue` instead of `agent.request_queue`, and set `turn_state.streaming = true` instead of `agent.streaming = true`.

4. **Tests `steering_delivery_resets_scroll` and `follow_up_delivery_resets_scroll`** - Fixed to push to `turn_state.message_queue` instead of `agent.message_queue`.

5. **Test `agent_error_delivers_queued_messages`** - Fixed to push to `turn_state.message_queue` instead of `agent.message_queue`.

6. **Test `new_turn_resets_speed`** - Fixed to set `turn_state` fields directly instead of `agent` fields.

## Acceptance Criteria

- [x] Unit tests — `AppState` projection rebuilds deterministically from `TurnState` events
- [x] E2E tests — No direct `agent_state_mut()` mutation in update handlers
- [x] Live run tests — Multi-tool turn updates state correctly

## Follow-up required

The 2026-07-03 architecture/code review found that `AppState` still stores a mutable `turn_state` field and production code mutates it directly in `update/agent/core_messages.rs`, `update/system.rs`, `update/dispatch.rs`, and `update/session.rs`. This violates the SSOT rule that `TurnState` is owned only by `TurnActor`.

See `tasks/remove-turnstate-from-appstate.md` and `tasks/close-turnstate-field-access-guardrail-gap.md` for the remaining work.
