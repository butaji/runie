# TurnActor owns agent turn lifecycle and queues

**Status**: in_progress
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection, session-actor-owns-session-state, input-actor-owns-input-state
**Blocks**: remove-direct-appstate-mutations

## Progress

**Completed:**
- ✅ TurnActor exists and owns TurnState with all required fields
- ✅ TurnMsg enum covers all required messages
- ✅ TurnActor emits facts: TurnStarted, TurnAborted, TurnCompleted, TurnErrored, TokenStatsUpdated, UserMessageSubmitted, QueueAborted, QueuesCleared
- ✅ Fact projection handlers in AppState: apply_turn_aborted, apply_turn_completed, apply_turn_errored, apply_token_stats, apply_turn_started, apply_user_message_submitted
- ✅ Fact event handlers in dispatch module
- ✅ Turn fact projections extracted to `model/state/turn_projections.rs`

**In progress:**
- Partial routing of handlers through TurnActor:
  - ✅ stop_turn() routes through TurnActor::AbortTurn
  - ✅ abort_queue() routes through TurnActor::AbortQueue  
  - ✅ queue_follow_up() routes through TurnActor::QueueFollowUp
  - ✅ handle_vim_dialog_back() routes through TurnActor
  - ✅ Agent events routed through TurnActor via handle_agent_event()
  - ✅ to_turn_msg() converts Event to TurnMsg for TurnActor routing
  - ✅ submit_user_message() routes through TurnActor::SubmitUserMessage

**Remaining work:**
- Queue delivery operations (deliver_queued, dequeue) - these need TurnActor coordination
- `update/system.rs` - peek_queue, pop_queue, configure_token_tracker - need routing

## Architecture Notes

The split between TurnActor and AppState is:
- **TurnActor**: owns queue state (request_queue, message_queue), turn lifecycle flags
- **AppState**: owns session.messages for UI projection, handles via facts from actors

Agent event handlers (set_thinking, start_tool, etc.) stay in AppState as they manage session.messages content.

## Description

`AgentState` turn flags, scheduling queues, and token accounting are mutated all over the UI layer and from the agent crate's handle. The `AgentActor` in `runie-agent` runs the provider turn but lets `UiActor` mutate all state. Extract turn lifecycle and scheduling into a dedicated `TurnActor`.

Current violators:
- `runie-agent/src/actor.rs::AgentActorHandle::run_if_queued` — sets `turn_active`, `inflight`, `streaming`, pops `request_queue`.
- `update/agent/core.rs` — `set_thinking`, `add_thought`, `start_tool`, `end_tool`, `append_response*`, `handle_llm_event`, `complete_turn`, `finish_turn`, `clear_turn_state`, `add_error`.
- `update/session.rs` — `queue_follow_up`, `abort_queue`, `deliver_queued`, `push_user_message`, `try_deliver_steering`, `dequeue`, `try_deliver_follow_up`.
- `update/system.rs` — `stop_turn`, `peek_queue`, `pop_queue`, `configure_token_tracker`.
- `update/input/text.rs` — `submit`, `dispatch_submit_content`.
- `update/input/mod.rs` — `handle_escape` calls `stop_turn`.
- `update/dialog_input.rs` — `handle_vim_dialog_back` resets turn flags.
- `commands/dsl/handlers/session/mod.rs` — `/new` clears queues.
- `model/cache.rs` — `update_speed` / `animate_tokens` use turn state.
- `model/state/app_state.rs` — `next_id`, `reset_session`, `apply_config`, `restore_session` touch token tracker and queues.

## Acceptance criteria

- [ ] `TurnActor` is an mpsc actor holding authoritative turn state: `turn_active`, `current_request_id`, `streaming`, `current_tool_name`, `current_action`, `inflight`, `turn_started_at`, `thinking_started_at`, `tool_started_at`, `intermediate_step_count`, `thought_seq`, `last_assistant_index`, `streaming_buffer`, `request_queue`, `message_queue`, `next_id`, `token_tracker`, `turn_tokens_out`, `tokens_in`, `tokens_out`, `speed_tps`, `speed_window`, `last_speed_update`, `tokens_at_last_speed`.
- [ ] `TurnMsg` covers: `RunIfQueued`, `AbortTurn`, `SubmitUserMessage { content }`, `QueueSteering { content }`, `QueueFollowUp { content }`, `AbortQueue`, `ClearQueues`, `Thinking { request_id }`, `ToolStart { ... }`, `ToolEnd { ... }`, `ResponseDelta { ... }`, `Done`, `Error { ... }`, `SetModel { provider, model }`, `UpdateSpeed`, `NextId`.
- [ ] `AppState.agent` fields for turn lifecycle are private; reads go through immutable accessors.
- [ ] `TurnActor` emits facts: `TurnStarted`, `TurnProgress`, `TurnCompleted`, `TurnAborted`, `TurnErrored`, `UserMessageAppended`, `SteeringDelivered`, `TokenStatsUpdated`.
- [ ] `runie-agent/src/actor.rs::AgentActorHandle::run_if_queued` no longer mutates `AppState`; it sends `TurnMsg::RunIfQueued`.
- [ ] `AgentActor` (provider runner) stays in `runie-agent`; it streams events that `TurnActor` consumes.
- [ ] Token display animation (`tokens_in_display`, `tokens_out_display`, `tokens_in_prev`, `tokens_out_prev`) stays in the UI layer (driven by `Tick` animation events).
- [ ] Token tracker is produced by `ProviderActor` or `ConfigActor` and passed to `TurnActor` on model switch.
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [ ] `turn_actor_run_if_queued_sets_active` — `RunIfQueued` with a queued request sets `turn_active=true` and emits `TurnStarted`.
- [ ] `turn_actor_abort_turn_emits_aborted` — `AbortTurn` clears flags and emits `TurnAborted`.
- [ ] `turn_actor_queue_lifecycle` — follow-up/steering queue drains produce `UserMessageAppended` + `RunIfQueued`.

### Layer 2 — Event Handling
- [ ] `submit_input_sends_submit_user_message` — Enter in chat sends `TurnMsg::SubmitUserMessage`.
- [ ] `agent_response_delta_routes_to_turn_actor` — LLM delta event is handled by `TurnActor`.

### Layer 3 — Rendering
- [ ] `turn_progress_updates_token_stats` — `TokenStatsUpdated` causes the status bar to render updated counts.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `mock_provider_multi_tool_turn_routes_through_turn_actor` — a full turn with tool calls produces correct messages and turn lifecycle with no direct `agent.*` writes outside the actor.

## Files touched

- `crates/runie-core/src/actors/turn/` — TurnActor implementation (mod.rs, messages.rs, state.rs, actor.rs).
- `crates/runie-core/src/actors/handles.rs` — added TurnActorHandle helpers.
- `crates/runie-core/src/model/state/app_state.rs` — private turn fields.
- `crates/runie-core/src/model/state/domain_ops.rs` — fact projection handlers.
- `crates/runie-core/src/update/agent/mod.rs` — dispatcher routes `AgentEvent` to `TurnActor`.
- `crates/runie-core/src/update/agent/core.rs` — agent lifecycle handlers.
- `crates/runie-core/src/update/dispatch.rs` — added handle_agent_event() and to_turn_msg() for TurnActor routing.
- `crates/runie-core/src/update/session.rs` — queue operations (partial).
- `crates/runie-core/src/update/system.rs` — `stop_turn` routes to TurnActor.
- `crates/runie-core/src/update/input/text.rs` — submit handler.
- `crates/runie-core/src/update/input/mod.rs` — escape handler.
- `crates/runie-core/src/update/dialog_input.rs` — vim dialog back routes to TurnActor.
- `crates/runie-core/src/commands/dsl/handlers/session/mod.rs` — `/new` emits `TurnMsg::ClearQueues`.
- `crates/runie-agent/src/actor.rs` — `AgentActorHandle` sends `TurnMsg`.
- `crates/runie-core/src/model/cache.rs` — token animation reads public turn stats only.

## Notes

- `TurnActor` decides *when* user/steering messages become part of the session; `SessionActor` decides *how* to store them.
- Assistant/tool/thought/turn-complete/error messages currently mutate `session.messages` inside `update/agent/core.rs`. Decide during implementation whether `TurnActor` owns these insertions (sending `SessionMsg` to `SessionActor`) or whether `SessionActor` owns the full message lifecycle. Document the split.
- `update_speed` in `model/cache.rs` currently mutates turn-state speed fields. Either move speed computation into `TurnActor` and have `ViewActor` read it, or keep speed as UI-derived state that reads public turn stats.
- Keep `AgentActor` as the provider-turn executor; do not move provider logic into `runie-core`.
