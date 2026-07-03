# Serialize projection update after actor commit

## Status

`todo`

## Description

Ensure `AppState` is updated only after `TurnActor` has committed and emitted facts. Fix the race in `dispatch.rs` and `ui_actor.rs:566-575`.

## Acceptance criteria

1. **Unit tests** — Projection update follows actor commit in deterministic order.
2. **E2E tests** — `run_if_queued` never runs on stale state.
3. **Live tmux tests** — Queue multiple turns and verify they run sequentially.

## Tests

### Unit tests
- Ordering test: actor commit → event → projection.

### E2E tests
- Queued turn replay.

### Live tmux tests
- Queue and run multiple turns.
