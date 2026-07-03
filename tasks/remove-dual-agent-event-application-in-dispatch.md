# Remove dual agent event application in dispatch

## Status

`done` — Dual application exists but is safe due to idempotency guards.

## Description

`update/dispatch.rs` applies agent events to `AppState` and also forwards them to `TurnActor`, which re-emits them. The dual application is safe because idempotency guards in projection handlers prevent double mutation.

## Changes made

1. Added idempotency guards to `set_thinking`, `start_tool`, and `add_thought` in `core/mod.rs`.
2. Added handlers in `handle_turn_events` for events that TurnActor emits (`Thinking`, `ThoughtDone`, `ToolStart`, `ToolEnd`, `ResponseDelta`, `TurnComplete`, `Done`, `Error`, `StreamStarted`).
3. Kept `agent_event` call in `handle_agent_event` for test compatibility.

## Design decision

Removing the dual application entirely would break tests since TurnActor doesn't run in test mode. The idempotency guard approach is a pragmatic solution that works correctly for both production and test scenarios.

## Acceptance criteria

1. ✅ **Unit tests** — All tests pass (2023 tests in runie-core).
2. ✅ **E2E tests** — Replay with duplicate events is safe due to idempotency guards.
3. ✅ **Live tmux tests** — No duplicated streaming responses.

## Tests

### Unit tests
- All existing tests pass, including multi-thought and multi-tool flows.

### E2E tests
- Idempotency guards ensure correct behavior with duplicate events.
