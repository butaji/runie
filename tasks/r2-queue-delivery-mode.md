# Queue delivery mode configuration

**Status**: done

**Milestone**: R2

**Category**: Input & Commands

## Description

Configure how queued messages are delivered to the LLM. Steering and follow-up modes can be set independently.

## Modes

- **one-at-a-time** (default): Each queued message triggers a separate LLM call. The agent waits for a response before processing the next message.
- **all**: All queued messages are delivered together in a single LLM call.

## Configuration

Set via `/settings` or `settings.json`:

```json
{
  "steeringMode": "one-at-a-time" | "all",
  "followUpMode": "one-at-a-time" | "all"
}
```

## Implementation

- `DeliveryMode` enum in `runie-core/src/model.rs`
- `steering_mode` and `follow_up_mode` fields on `AppState`
- `deliver_queued()` in `runie-core/src/update/queue.rs` respects delivery mode
- `finish_turn()` in `runie-core/src/update/agent.rs` triggers `deliver_queued()` on `AgentDone`

## Acceptance Criteria

- [x] steeringMode configuration (one-at-a-time / all)
- [x] followUpMode configuration (one-at-a-time / all)
- [x] `deliver_queued()` moves messages from `message_queue` to `request_queue` per mode
- [x] `finish_turn()` calls `deliver_queued()` after `AgentDone`
- [x] Default to one-at-a-time for both modes

## Tests

- [x] Layer 1 — State/logic: `tests/queue.rs` delivery mode tests
- [x] Layer 2 — Event handling: AgentDone triggers delivery per mode
- [x] Layer 3 — Rendering: N/A (logic only)
- [x] Layer 4 — Smoke: Verified manually
