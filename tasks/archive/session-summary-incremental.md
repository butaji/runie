# Update Session Summary Incrementally

**Status**: done
**Milestone**: R3
**Category**: Sessions
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

On every `MessageSent` durable event, `SessionActor` calls `store.load_events(&self.session_id)` to rebuild a 500-character summary. This is O(n) disk I/O per message.

## Acceptance Criteria

- [x] Summary state is tracked incrementally in memory.
- [x] Updates use only the new event content, not a full reload.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `summary_updated_without_reloading_events` — summary changes after appending an event without calling `load_events`.

### Layer 2 — Event Handling
- [x] `message_sent_updates_summary` — summary updates on `MessageSent`.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/src/session_actor.rs`

## Notes

This pairs with `session-store-blocking-io`: reducing disk access makes the actor more efficient even before spawn_blocking is applied.
