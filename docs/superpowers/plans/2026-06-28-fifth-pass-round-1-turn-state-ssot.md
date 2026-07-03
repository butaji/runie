# Round 1 — TurnState / AgentState SSOT

## Findings

### 1. Two turn state machines update `AppState`

`crates/runie-core/src/update/dispatch.rs:137-144` forwards agent events to `TurnActor` **and also** applies them to `AppState` via `agent_event(state, event)`. `TurnActor` then re-emits the same facts, so `AppState` applies them twice.

For `Thinking`, this creates a feedback loop: `handle_thinking` in `crates/runie-core/src/actors/turn/handlers.rs:178` re-emits `Event::Thinking`, which is dispatched again.

### 2. `AgentState` and `TurnState` diverge

Because `AgentState` is rebuilt from the local `AppState.turn_state` copy, not from `TurnActor`'s authoritative state:

- `inflight` is incremented in both `turn_projections.rs:214` and `handlers.rs:27`.
- `request_queue` is popped in `handlers.rs:24` but never popped in the projection.
- `tokens_in` estimate is added in `update/input/submit.rs:75-78` only to `AppState`.
- `token_tracker` is configured in `update/system.rs:115-119` only in `AgentState`.
- `speed_tps` / `speed_window` are updated in `model/cache/animation.rs:64-92` only in `AgentState`.
- `crates/runie-tui/src/ui_actor/mod.rs:552` forces `turn_active = false` in `AgentState` regardless of `TurnState`.

### 3. Direct lifecycle mutations outside `TurnActor`

- `crates/runie-core/src/model/state/domain_ops.rs:78-83` — `next_id()` mutates `turn_state.next_id` without an event.
- `crates/runie-core/src/model/state/app_state.rs:64-73` — `set_turn_active` / `set_streaming` mutate turn state directly (test helpers leaking into production).
- `crates/runie-core/src/update/input/submit.rs:75-78` — `estimate_and_add_tokens` updates token count without an event.
- `crates/runie-core/src/update/system.rs:115-119` — `configure_token_tracker` mutates `agent.token_tracker` but not `turn_state.token_tracker`.
- `crates/runie-core/src/model/cache/animation.rs:64-92` — `update_speed` patches `AgentState` directly.

## Recommended changes

1. Make `TurnActor` the sole owner of turn/queue/token/inflight state.
2. Remove `AppState::start_turn`, `AppState::next_id`, and similar direct lifecycle mutators.
3. Emit `InputTokensEstimated`, `TokenTrackerConfigured`, `SpeedUpdated` facts from the appropriate actors.
4. Treat `AgentState` as a pure `From<&TurnState>` projection.
5. Stop `UiActor` from forcing `turn_active`; react to `TurnCompleted`/`TurnErrored`/`TurnAborted` events.

## Task mapping

| Finding | Task file | Status |
|---------|-----------|--------|
| Make `TurnActor` sole owner of turn state | `tasks/make-turnactor-sole-owner-of-turn-state.md` | **new** |
| Remove dual agent event application | `tasks/remove-dual-agent-event-application-in-dispatch.md` | **new** |
| Remove direct turn lifecycle mutations | `tasks/remove-direct-turn-lifecycle-mutations-outside-turnactor.md` | **new** |
| Treat `AgentState` as pure projection | `tasks/treat-agentstate-as-pure-turnstate-projection.md` | **new** |
