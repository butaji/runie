# Serialize projection update after actor commit

## Status

`done`

## Description

Projection updates are serialized via idempotency guards in projection handlers. The architecture uses `apply_and_order` for atomic state changes, and idempotent guards prevent double mutation when events are applied both directly (in `agent_event`) and via TurnActor facts (in `handle_turn_events`).

## How it works

1. **`apply_and_order`** — Wraps each projection call to also call `ensure_turn_complete_last()`, ensuring turn complete messages stay last.

2. **Idempotency guards** — Projection methods check before mutating:
   - `set_thinking` — skips if already streaming with same request_id
   - `add_thought` — skips if thought already created at current seq
   - `start_tool` — skips if already running this tool
   - `end_tool` — clears tool state (idempotent)

3. **Dual application** — Events from TurnActor are applied both:
   - Directly via `agent_event()` in `handle_agent_event`
   - Via `handle_turn_events()` when TurnActor emits facts

   The idempotency guards make the second application a no-op, preventing double mutation.

4. **`ui_actor.rs`** — Uses RPC (`deliver_queued`) to ensure TurnActor processes before the next step, eliminating the previous polling race.

## Acceptance criteria

1. ✅ **Unit tests** — Idempotency guards prevent double application; ordering preserved.
2. ✅ **E2E tests** — `run_if_queued` works correctly with idempotent guards.
3. ✅ **Live tmux tests** — Queue multiple turns and verify sequential execution.
