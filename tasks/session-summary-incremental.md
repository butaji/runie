# Update Session Summary Incrementally

**Status**: todo
**Milestone**: R3
**Category**: Sessions
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

On every `MessageSent` durable event, `SessionActor` calls `store.load_events(&self.session_id)` to rebuild a 500-character summary. This is O(n) disk I/O per message.

## Acceptance Criteria

- [ ] Summary state is tracked incrementally in memory.
- [ ] Updates use only the new event content, not a full reload.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `summary_updated_without_reloading_events` — summary changes after appending an event without calling `load_events`.

### Layer 2 — Event Handling
- [ ] `message_sent_updates_summary` — summary updates on `MessageSent`.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/src/session_actor.rs`

## Notes

This pairs with `session-store-blocking-io`: reducing disk access makes the actor more efficient even before spawn_blocking is applied.
