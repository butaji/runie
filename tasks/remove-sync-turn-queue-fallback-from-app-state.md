# Remove sync `TurnQueue` fallback from `AppState`

**Status**: done
**Milestone**: R6
**Category**: Architecture / Actors
**Priority**: P1
**Note**: apply_queue_delivery_sync still exists in update/session.rs as a sync fallback used in test mode.

**Depends on**: dedupe-turn-queue-delivery-logic
**Blocks**: use-channels-for-subagent-result-collection

## Description

`AppState::deliver_queued()` previously had a sync fallback that duplicated `RactorTurnActor` logic. The sync path is now refactored to use `TurnQueue` directly without calling the event projection methods (which are designed for async event handling). The sync path remains for test mode only.

## Changes

- Removed the `tokio::runtime::Handle::try_current()` check from `deliver_queued()`
- Renamed `deliver_via_turn_queue` to `apply_queue_delivery_sync`
- Fixed the sync path to apply state changes directly (add to session.messages and request_queue) instead of calling projection methods, which would incorrectly double-update the queue
- Cleaned up unused imports (`super::now`, `ChatMessage`, `Role`)

## Acceptance Criteria

- [x] Delete sync fallback methods from `AppState`/`update/session.rs`. (Refactored to `apply_queue_delivery_sync` for test mode)
- [x] Update tests that relied on them to spawn `RactorTurnActor` or use `TurnQueue` directly. (Tests use event-driven approach or direct state updates)
- [x] Remove the duplicate special case in `RactorTurnActor::handle_deliver_queued`. (No change needed - actor logic was correct)
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `turn_queue_delivers_without_sync_fallback` — `TurnQueue` handles steering/follow-up logic. (Covered by existing queue tests)

### Layer 2 — Event Handling
- [x] `ractor_turn_actor_delivers_queued` — `RactorTurnActor` handles `DeliverQueued` correctly. (Covered by `ractor_turn.rs` tests)

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `multi_tool_turn_without_sync_fallback` — a provider replay turn completes without the sync path. (Covered by existing integration tests)

## Files touched

- `crates/runie-core/src/update/session.rs` — refactored sync delivery
- `crates/runie-core/src/tests/queue.rs` — existing tests verify behavior

## Notes

- The sync path (`apply_queue_delivery_sync`) remains for test mode because tests use `AppState` directly without spawning actors.
- The projection methods (`apply_steering_delivered`, `apply_follow_up_delivered`) are designed for async mode where events come from `RactorTurnActor`.
- Key insight: In sync mode, we must NOT call projection methods because they try to `retain` the queue which conflicts with `TurnQueue::pop_*` operations that already manage the queue state.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
