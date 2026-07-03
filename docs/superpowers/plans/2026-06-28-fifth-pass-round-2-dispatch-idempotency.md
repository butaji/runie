# Round 2 — Event Dispatch and Idempotency

## Findings

### 1. Projection handlers are not idempotent

| Handler | Location | Effect of duplicate event |
|---------|----------|---------------------------|
| `apply_turn_started` | `model/state/turn_projections.rs:211-219` | `inflight` increments again. |
| `set_thinking` | `update/agent/core/mod.rs:9-31` | Resets `streaming_buffer`, `turn_tokens_out`, speed tracking; buffered content lost. |
| `start_tool` | `update/agent/core/mod.rs:83-114` | Appends another placeholder tool message. |
| `append_response` / `on_response_delta` | `update/agent/core/mod.rs:146-159`, `:295-317` | Duplicate text appended to assistant message. |
| `finish_turn` | `update/agent/core_messages.rs:104-130` | Redelivers queued messages. |
| `add_error` | `update/agent/core/mod.rs:251-278` | Pushes another error message. |
| `TokenStatsUpdated` → `CompactionTriggered` | `update/dispatch.rs:86-108` | Compacts twice. |
| `apply_queue_aborted` | `update/system.rs:199-207` | Appends content to input repeatedly. |

### 2. Derived values carried in events

| Event | Derived field | Location |
|-------|---------------|----------|
| `TokenStatsUpdated` | `speed_tps` | `actors/turn/handlers.rs:247` |
| `TurnComplete` | `duration_secs` | `runie-agent/src/turn/mod.rs:128` |
| `CompactionTriggered` | `ratio`, `tokens_in`, `context_window` | `update/dispatch.rs:93-104` |
| `StreamStarted` | derived from first `ResponseDelta` | `actors/turn/handlers.rs:210-216` |

Derived values break replay: the same raw facts produce different derived values depending on when they are observed.

### 3. Missing fact events

- `QueueFollowUp` / `QueueSteering` added in `actors/turn/handlers.rs:80-93` — `AppState` never learns the queue changed.
- `QueuesCleared` — no projection handler in `dispatch.rs`.
- `QueueAborted` — appends content but never removes the message from `message_queue`.
- Input token estimation — no fact emitted.
- ID allocation — no fact emitted.
- Speed update — no fact emitted; `AgentState` patched directly.

## Recommended changes

1. Add stable `request_id` / `turn_id` to all turn/submission events.
2. Make projection handlers idempotent by checking if the fact is already applied for the given ID.
3. Move speed, duration, and compaction thresholds out of events; compute them in projection code.
4. Add missing queue fact events (`QueueFollowUpAdded`, `QueueSteeringAdded`, `QueuesCleared`, `QueueAborted`).
5. Ensure every `TurnMsg` that changes state emits a corresponding fact.

## Task mapping

| Finding | Task file | Status |
|---------|-----------|--------|
| Add idempotency keys to turn events | `tasks/add-idempotency-keys-to-turn-events.md` | **new** |
| Make projection handlers idempotent | `tasks/make-projection-handlers-idempotent.md` | **new** |
| Move derived values out of events | `tasks/move-derived-values-out-of-events.md` | **new** |
| Add missing queue fact events | `tasks/add-missing-queue-fact-events.md` | **new** |
