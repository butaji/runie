# Dedupe turn-queue delivery logic

**Status**: done
**Milestone**: R2
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: migrate-production-actors-to-ractor, collapse-actor-handles-to-typed-map
**Blocks**: none

## Description

`crates/runie-core/src/actors/turn/ractor_turn.rs` and `crates/runie-core/src/update/session.rs` both implement the same turn-queue delivery semantics (`try_deliver_steering`, `try_deliver_follow_up`, `deliver_follow_ups_all` vs. sync variants). The two implementations encode the same `DeliveryMode` semantics but drift in naming and edge cases. Extracting a single pure `TurnQueue` struct (or routing all queue operations through `RactorTurnActor`) removes ~100–140 lines and eliminates drift.

## Acceptance Criteria

- [x] Extract a pure, tested `TurnQueue` struct that owns `pop_steering(mode)`, `pop_follow_up(mode)`, and `pop_all_follow_ups()`.
- [x] Use the same `TurnQueue` from both `RactorTurnActor` and the sync test fallback, or drop the sync fallback and spawn `RactorTurnActor` in tests.
- [x] Preserve `DeliveryMode` semantics (OneAtATime vs All, Steering vs FollowUp priority).
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `turn_queue_pop_steering_one_at_a_time` — only the highest-priority steering item is popped.
- [x] `turn_queue_pop_all_follow_ups` — all follow-ups are returned in priority order.
- [x] `turn_queue_empty_after_clear` — no items remain after clearing.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `turn_delivery_replay_matches` — existing provider replay fixtures still produce the same turn sequence.

## Files touched

- `crates/runie-core/src/actors/turn/ractor_turn.rs`
- `crates/runie-core/src/update/session.rs`
- New `crates/runie-core/src/session/turn_queue.rs` (or similar)

## Notes

- Coordinate with `collapse-actor-handles-to-typed-map.md` because `RactorTurnActor` handle wiring may change.
- If the sync fallback is kept only for tests, consider moving it to `runie-testing`.
- **Update after review:** a sync fallback still exists in `crates/runie-core/src/update/session.rs`. Removing it is tracked by `remove-sync-turn-queue-fallback-from-app-state.md`.
