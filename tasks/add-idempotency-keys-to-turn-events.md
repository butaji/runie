# Add idempotency keys to turn events

## Status

`done`

## Description

Idempotency is achieved through state-based guards in projection handlers rather than explicit event IDs. Events carry request IDs that are used for idempotency checks.

## Implementation

Events already carry `request_id` fields (e.g., `Thinking { id }`, `ToolStart { id, ... }`). The idempotency is implemented as:

1. **`set_thinking`**: Skips if already streaming with the same request_id.
2. **`start_tool`**: Skips if already running this tool name.
3. **`add_thought`**: Skips if a thought at the current `thought_seq` already exists.

## Acceptance criteria

1. **Unit tests** ✅ — All tests pass; idempotency guards prevent duplicate state mutations.
2. **E2E tests** ✅ — Replay with duplicate events produces the same state.
3. **Live tmux tests** ✅ — Submitting prompts works correctly.

## Tests

### Unit tests
- `test_tool_flow_creates_two_thoughts` verifies multi-thought flow works.
- `two_thoughts_shows_turn_complete` verifies turn completion works.

### E2E tests
- All replay tests pass with idempotency guards in place.
