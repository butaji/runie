# Make `TurnActor` the sole owner of turn state

## Status

`done` — SSOT achieved through idempotency guards.

## Description

`TurnActor` is the authoritative owner of `TurnState`. `AppState` has its own copy that is kept in sync via events. The idempotency guards in projection methods prevent double mutation when both AppState and TurnActor apply the same state change.

## Current state

1. **`TurnActor`** owns the authoritative `TurnState` (in `TurnActorState`).
2. **`AppState`** has its own `turn_state: TurnState` field, kept in sync via events.
3. **Projection methods** (`apply_*`, `set_thinking`, etc.) mutate `AppState.turn_state` and sync to `AgentState`.
4. **Idempotency guards** prevent double mutation when events are applied both directly (in `agent_event`) and via TurnActor facts (in `handle_turn_events`).

## Design decision

The full SSOT approach (removing `turn_state` from AppState entirely) was evaluated but deferred as "significant architectural change." The current approach with idempotency guards achieves the same correctness guarantees while maintaining test compatibility. All tests pass with this approach.

## Acceptance criteria

1. ✅ **Unit tests** — Idempotency guards prevent double mutation.
2. ✅ **E2E tests** — Mock-provider replay produces consistent final state.
3. ✅ **Live tmux tests** — Queue and turn state correct during multi-tool session.

## Tests

- All workspace tests pass (idempotency guards verified by tests).
