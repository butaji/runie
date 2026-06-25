# Drop Event Bus Replay Buffer

**Status**: todo
**Milestone**: R4
**Category": Architecture / Refactoring
**Priority**: P3

**Depends on**: event-taxonomy-for-actor-state-sync
**Blocks**: drop-small-stdlib-replaceable-deps

## Description

Remove the replay buffer from the event bus. The event bus currently maintains a buffer of recent events for replay, but this can be simplified by relying on `SessionActor` for durable event storage.

## Acceptance Criteria

- [ ] Event bus replay buffer removed
- [ ] Event bus still broadcasts to subscribers
- [ ] Session replay works via `SessionActor`
- [ ] `cargo test --workspace` passes

## Tests

### Layer 1 — State/Logic
- [ ] `event_bus_without_replay_buffer`

### Layer 2 — Event Handling
- [ ] `events_broadcast_to_all_subscribers`

### Layer 3 — Rendering
- [ ] N/A

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `session_replay_works_via_session_actor`

## Files touched

- `crates/runie-core/src/bus.rs`

## Notes

- Simplification task
- Session durability is handled by `SessionActor`
