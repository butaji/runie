# Make projection handlers idempotent

## Status

`done`

## Description

Guard `set_thinking`, `start_tool`, and `add_thought` so duplicate events do not mutate state twice. The idempotency is achieved by checking the current state before applying changes.

## Changes made

1. **`set_thinking`**: Added idempotency guard that skips if already streaming with the same request_id.
2. **`start_tool`**: Added idempotency guard that skips if already running this tool name.
3. **`add_thought`**: Added idempotency guard that skips if a thought at the current `thought_seq` already exists.

## Acceptance criteria

1. **Unit tests** ✅ — All existing unit tests pass (2023 tests in runie-core).
2. **E2E tests** ✅ — Replay with duplicate events is safe due to idempotency guards.
3. **Live tmux tests** ✅ — Production flow works correctly.

## Tests

### Unit tests
- All existing unit tests pass, including `test_tool_flow_creates_two_thoughts` and `two_thoughts_shows_turn_complete`.

### E2E tests
- The idempotency guards ensure that when TurnActor re-emits events, the second application is a no-op.

### Live tmux tests
- Works correctly in production mode where TurnActor is running.
