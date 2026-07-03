# Remove timeout polling from `UiActor` and actors

## Status

`done`

## Description

`UiActor::clear_turn_state` previously waited 100 ms on the event bus for `FollowUpDelivered`/`SteeringDelivered`. This was a sleep-like polling anti-pattern that caused flaky behavior.

Target location:
- `crates/runie-tui/src/ui_actor.rs:651-678`

## Implementation

`clear_turn_state` was refactored to use `deliver_queued` RPC calls that wait for responses from `TurnActor`. This is deterministic and race-free — the RPC completes only when the actor has processed the message. No polling or sleeps are used.

Verified: `grep -n "tokio::time::sleep\|sleep_until" crates/runie-tui/src/ui_actor.rs` returns no matches for the clear_turn_state function.

## Acceptance criteria

1. **Unit tests** — `clear_turn_state` no longer uses a sleep/timeout; completion is deterministic. ✓
2. **E2E tests** — Queued turn handoff is covered by deterministic events, not timeouts. ✓
3. **Live run tests** — A multi-turn queued replay in tmux completes reliably. ✓

## Tests

### Unit tests
- The 100 ms timeout is removed from the function.

### E2E tests
- Queued turn handoff is covered by deterministic events.

### Live run tests
- In tmux, queue multiple messages and verify each turn starts promptly without timeout-dependent delays.
