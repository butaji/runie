# Remove timeout polling from `UiActor` and actors

## Status

`todo`

## Description

`UiActor::clear_turn_state` waits 100 ms on the event bus for `FollowUpDelivered`/`SteeringDelivered`. This is a sleep-like polling anti-pattern that causes flaky behavior.

Target location:
- `crates/runie-tui/src/ui_actor.rs:651-678`

## Acceptance criteria

- The 100 ms timeout is removed.
- `UiActor` either receives an explicit completion event or holds a future from `TurnActor`.

## Tests

### Layer 2 — Event Handling
- Queued turn handoff is covered by deterministic events, not timeouts.

### Layer 4 — Provider Replay / Mock-Tool E2E
- Multi-turn queued replay completes reliably without sleeps.
