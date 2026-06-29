# Remove sync `TurnQueue` fallback from `AppState`

**Status**: todo
**Milestone**: R6
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: dedupe-turn-queue-delivery-logic
**Blocks**: use-channels-for-subagent-result-collection

## Description

`AppState` contains synchronous mirror methods (`apply_deliver_queued_sync`, `try_deliver_steering_sync`, `try_deliver_follow_up_sync`, `try_deliver_follow_ups_all_sync`) that duplicate the async `TurnQueue` logic. The deduplication task left this fallback in place. Remove it and route tests through `RactorTurnActor` so `TurnQueue` owns all delivery semantics.

## Acceptance Criteria

- [ ] Delete sync fallback methods from `AppState`/`update/session.rs`.
- [ ] Update tests that relied on them to spawn `RactorTurnActor` or use `TurnQueue` directly.
- [ ] Remove the duplicate special case in `RactorTurnActor::handle_deliver_queued`.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `turn_queue_delivers_without_sync_fallback` — `TurnQueue` handles steering/follow-up logic.

### Layer 2 — Event Handling
- [ ] `ractor_turn_actor_delivers_queued` — `RactorTurnActor` handles `DeliverQueued` correctly.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `multi_tool_turn_without_sync_fallback` — a provider replay turn completes without the sync path.

## Files touched

- `crates/runie-core/src/update/session.rs`
- `crates/runie-core/src/actors/turn/ractor_turn.rs`
- `crates/runie-core/src/session/turn_queue.rs`
- `crates/runie-core/src/tests/queue.rs`

## Notes

- This unblocks `use-channels-for-subagent-result-collection.md` by removing a second delivery path.
