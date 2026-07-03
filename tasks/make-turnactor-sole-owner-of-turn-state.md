# Make `TurnActor` the sole owner of turn state

## Status

`partial` — SSOT achieved through idempotency guards; parallel mutation paths remain but are safe.

## Description

`TurnActor` is the authoritative owner of `TurnState`. `AppState` has its own copy that is kept in sync via events. The idempotency guards in projection methods prevent double mutation when both AppState and TurnActor apply the same state change.

## Current state

1. **`TurnActor`** owns the authoritative `TurnState` (in `TurnActorState`).
2. **`AppState`** has its own `turn_state: TurnState` field, kept in sync via events.
3. **Projection methods** (`apply_*`, `set_thinking`, etc.) mutate `AppState.turn_state` and sync to `AgentState`.
4. **Idempotency guards** prevent double mutation when events are applied both directly (in `agent_event`) and via TurnActor facts (in `handle_turn_events`).

## What remains

- `AppState` still owns `turn_state` (a copy of TurnActor's authoritative state)
- AppState projection methods directly mutate `turn_state` instead of receiving projected values from events
- To fully implement SSOT, `AppState` should NOT own `turn_state` and should only own `AgentState` (the read-only projection)

This would require:
1. Remove `turn_state` field from `AppState`
2. Add projected fields directly to `AppState` that are updated only via events
3. Remove all `turn_state_mut()` calls from production code
4. Update all projection methods to work with projected fields instead of authoritative fields

This is a significant architectural change tracked as future work.

## Acceptance criteria

1. ✅ **Unit tests** — Idempotency guards prevent double mutation.
2. ✅ **E2E tests** — Mock-provider replay produces consistent final state.
3. ✅ **Live tmux tests** — Queue and turn state correct during multi-tool session.

## Tests

- All workspace tests pass (idempotency guards verified by tests).
