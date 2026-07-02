# Instrument actor handlers with tracing

## Status

`todo`

## Description

Actor handlers in `TurnActor`, `ProviderActor`, `SessionActor` are uninstrumented. Add `#[tracing::instrument]` with span fields for `turn_id`, `provider`, `model`.

## Acceptance criteria

1. **Unit tests** — Tests verify spans are created with correct fields.
2. **E2E tests** — Actor replay tests still pass.
3. **Live tmux tests** — Run with debug logging and observe actor spans.

## Tests

### Unit tests
- `tracing_test` asserts span fields.

### E2E tests
- Replay turn exercises instrumented actors.

### Live tmux tests
- Submit a prompt and check logs for actor spans.
