# Add missing queue fact events

## Status

`done`

## Description

Added `QueueFollowUpAdded`, `QueueSteeringAdded`, and fixed `handle_clear_queues` to emit `QueueAborted` for each message before clearing.

## Changes

1. **Event definitions** — Added `Event::QueueFollowUpAdded { id, content }` and `Event::QueueSteeringAdded { id, content }` to `event/mod.rs`. Added `EventKind::Fact` and `EventCategory::Agent` classifications. Added to durable conversion (returns `None` — transient events).

2. **TurnActor handlers** — `handle_queue_steering` and `handle_queue_follow_up` now emit `QueueSteeringAdded` and `QueueFollowUpAdded` events respectively, using queue length as the ID.

3. **`handle_clear_queues`** — Now drains messages and emits `QueueAborted` for each before emitting `QueuesCleared`. This ensures AppState gets notified about each cleared message.

4. **Dispatch handlers** — Added `Event::QueueFollowUpAdded` and `Event::QueueSteeringAdded` to `handle_turn_events` in `dispatch.rs`.

5. **Projection methods** — Added `apply_queue_follow_up_added` and `apply_queue_steering_added` to `turn_projections.rs`. Updated `queue_follow_up` in `session.rs` and `queue_steering_message` in `submit.rs` to use the projection methods.

6. **Bug fix** — `queue_steering_message` was incorrectly sending `TurnMsg::QueueFollowUp` (should be `TurnMsg::QueueSteering`). Fixed.

7. **Tests** — Added `apply_queue_follow_up_added`, `apply_queue_steering_added`, and `apply_queue_follow_up_added_multiple` tests.

## Acceptance criteria

1. ✅ **Unit tests** — Queue fact projection tests pass.
2. ✅ **E2E tests** — Replay with queued messages works.
3. ✅ **Live tmux tests** — Queue, abort, and clear messages in tmux (manual).

## Tests

### Unit tests
- `apply_queue_follow_up_added` — adds follow-up to queue
- `apply_queue_steering_added` — adds steering to queue
- `apply_queue_follow_up_added_multiple` — adds multiple follow-ups in order
