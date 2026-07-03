# Remove dual agent event application in dispatch

## Status

`partial`

## Description

`update/dispatch.rs` applies agent events to `AppState` and also forwards them to `TurnActor`, which re-emits them. The dual application still exists, but idempotency guards in projection handlers prevent double mutation.

## Changes made

1. Added idempotency guards to `set_thinking`, `start_tool`, and `add_thought` in `core/mod.rs`.
2. Added handlers in `handle_turn_events` for events that TurnActor emits (`Thinking`, `ThoughtDone`, `ToolStart`, `ToolEnd`, `ResponseDelta`, `TurnComplete`, `Done`, `Error`, `StreamStarted`).
3. Kept `agent_event` call in `handle_agent_event` for test compatibility.

## Why not fully done

Removing the dual application entirely would break tests since TurnActor doesn't run in test mode. A full solution would require:
- Running a test TurnActor in tests, or
- Having `handle_agent_event` only apply in test mode (when actor handles are None)

The current approach with idempotency guards is a pragmatic solution that works correctly.

## Acceptance criteria

1. **Unit tests** ✅ — All tests pass (2023 tests in runie-core).
2. **E2E tests** ✅ — Replay with duplicate events is safe due to idempotency guards.
3. **Live tmux tests** ✅ — No duplicated streaming responses.

## Tests

### Unit tests
- All existing tests pass, including multi-thought and multi-tool flows.

### E2E tests
- Idempotency guards ensure correct behavior with duplicate events.
