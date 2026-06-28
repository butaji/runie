# Deduplicate turn-queue delivery logic

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: none
**Blocks**: use-channels-for-subagent-result-collection

## Description

The agent turn queue and event dispatch layer have overlapping buffering and deduplication logic. This produces stale indices, duplicate `TurnComplete` events, and leaked in-flight turns. Unify them into a single queue that owns pending events and deduplicates by an explicit delivery identifier.

## Acceptance Criteria

- [ ] There is exactly one turn queue component responsible for ordering and delivery.
- [ ] Each emitted event carries a delivery id; duplicates are dropped.
- [ ] No duplicate `TurnComplete` events reach consumers.
- [ ] In-flight turns are cleaned up on cancellation or actor stop.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `queue_drops_duplicate_delivery_id` — duplicate events with the same id are ignored.
- [ ] `queue_cancels_inflight` — cancellation removes pending and in-flight entries.

### Layer 2 — Event Handling
- [ ] `turn_event_dispatches_once` — a turn result is delivered exactly once to subscribers.

### Layer 3 — Rendering
- [ ] N/A — queue logic has no direct TUI output.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `multi_tool_turn_single_complete` — a multi-tool provider replay emits exactly one `TurnComplete`.

## Files touched

- `crates/runie-agent/src/turn.rs`
- `crates/runie-agent/src/queue.rs`
- `crates/runie-agent/src/dispatch.rs`
- `crates/runie-agent/src/actor.rs`

## Notes

- Consider using `tokio::sync::mpsc` with bounded channels for backpressure after this task.
- Keep the queue actor-independent; it should be testable without ractor.
