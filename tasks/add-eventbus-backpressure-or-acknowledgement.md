# Add EventBus backpressure or acknowledgement

**Status**: done
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: subscribe-tui-to-initial-facts-before-leader-start
**Blocks**: fix-tui-mock-simple-text-response-repetition

## Description

`EventBus` uses `tokio::sync::broadcast` with capacity `max(1) * 2`. A lagging subscriber (e.g. a slow render path or a temporarily busy actor) causes events to be dropped silently. This can lose `TurnComplete`, `Done`, `TurnErrored`, or streaming deltas, leaving the UI stuck.

## Root Cause

The broadcast channel has a tiny buffer and no backpressure or acknowledgement mechanism. `EventBus::publish` also swallows send errors with `unwrap_or(0)`.

## Acceptance Criteria

- [ ] The event bus capacity is sized for realistic burst traffic or uses a different channel type that preserves ordering.
- [ ] Publishers can detect when there are no subscribers (or log a warning).
- [ ] Critical control events (`TurnComplete`, `Done`, `TurnErrored`, `AbortTurn`) are not dropped under normal load.
- [ ] `cargo test --workspace` passes.
- [ ] Live tmux stress test (rapid input or fast deltas) does not miss completion events.

## Tests

### Layer 1 — State/Logic
- [ ] `eventbus_does_not_drop_control_events` — publish many events to a slow consumer and assert critical events are received.

### Layer 2 — Event Handling
- [ ] `publisher_warns_on_zero_subscribers` — `publish` returns/logs when `subscriber_count() == 0`.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A — behavior covered by unit/e2e tests; live tmux smoke test is a sanity check.

## Files touched

- `crates/runie-core/src/bus.rs`
- `crates/runie-core/src/actors/leader/actor.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- Possible fixes: increase buffer, split control events onto a dedicated channel, or replace broadcast with a fan-out pattern that awaits slow consumers.
- Coordinate with the late-subscription fix; both touch the bus and startup ordering.
