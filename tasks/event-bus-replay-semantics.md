# EventBus Replay Buffer Semantics

**Status**: todo
**Milestone**: R3
**Category**: Core / State
**Priority**: P0

**Depends on**: none
**Blocks**: `session-replay-startup-ordering`

## Description

`ReplayBuffer::drain` uses `VecDeque::drain(..)`, which empties the buffer. The first `subscribe_with_replay` consumes all replay events; any subsequent late subscriber gets an empty replay. This breaks the documented "bounded replay buffer for late subscribers" semantics and can cause state-loss for subscribers that join after another.

## Acceptance Criteria

- [ ] Late subscribers each receive a copy of the buffered replay events.
- [ ] The replay buffer is bounded and evicts old events when the capacity is exceeded.
- [ ] Existing tests still pass.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `replay_buffer_clone_does_not_drain` — two subscribers both receive history.
- [ ] `replay_buffer_respects_capacity` — oldest events are dropped when buffer overflows.

### Layer 2 — Event Handling
- [ ] `second_subscriber_receives_replay` — subscribe twice; both get the same replay.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/src/bus.rs`

## Notes

Fix is a one-line change: clone the buffer contents instead of draining. Also evaluate replacing the `std::sync::Mutex` with `parking_lot::Mutex` to avoid poisoning panics (see `event-bus-poisoned-mutex`).
