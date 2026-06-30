# Remove runtime presence branching in submit user message

**Status**: todo
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: remove-direct-appstate-mutation-from-tui-handlers
**Blocks**: fix-tui-turn-complete-leaves-working-status-and-queued

## Description

`submit_user_message` branches on `tokio::runtime::Handle::try_current()` to decide whether to send an actor message or mutate state directly. This means tests running under `#[tokio::test]` take the “production” path even when no actors are spawned, causing surprising and inconsistent behavior.

## Root Cause

The code uses runtime presence as a proxy for “are actors available?”.

## Acceptance Criteria

- [ ] The code branches only on `state.actor_handles().is_some()`.
- [ ] Tests that do not spawn actors take the synchronous/state-only path.
- [ ] Tests with actor handles take the production path.
- [ ] `cargo test --workspace` passes.
- [ ] Live tmux message submission still works.

## Tests

### Layer 1 — State/Logic
- [ ] `submit_without_actors_uses_sync_path` — `AppState` without actor handles updates directly.
- [ ] `submit_with_actors_sends_turn_msg` — `AppState` with actor handles emits `TurnMsg::SubmitUserMessage`.

### Layer 2 — Event Handling
- [ ] `submit_event_under_tokio_without_actors_is_sync` — `#[tokio::test]` with no leader uses the sync path.

## Files touched

- `crates/runie-core/src/update/input/submit.rs`
- `crates/runie-core/src/model/app_state.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- Removing this branching is part of making the architecture deterministic and testable.
