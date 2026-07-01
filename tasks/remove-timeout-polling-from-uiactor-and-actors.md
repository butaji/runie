# Remove timeout polling from `UiActor` and actors

## Status

`todo`

## Description

`UiActor::clear_turn_state` waits 100 ms on the event bus for `FollowUpDelivered`/`SteeringDelivered`. This is a sleep-like polling anti-pattern that causes flaky behavior.

Target location:
- `crates/runie-tui/src/ui_actor.rs:651-678`

## Acceptance criteria

1. **Unit tests** — `clear_turn_state` no longer uses a sleep/timeout; completion is deterministic.
2. **E2E tests** — Queued turn handoff is covered by deterministic events, not timeouts.
3. **Live run tests** — A multi-turn queued replay in tmux completes reliably.

## Tests

### Unit tests
- The 100 ms timeout is removed from the function.

### E2E tests
- Queued turn handoff is covered by deterministic events.

### Live run tests
- In tmux, queue multiple messages and verify each turn starts promptly without timeout-dependent delays.
