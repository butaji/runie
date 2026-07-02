# Add tracing to `runie-provider`

## Status

`todo`

## Description

`runie-provider` currently has no tracing. Add spans around retries, validation, SSE parsing, and request building.

## Acceptance criteria

1. **Unit tests** — `tracing_mock` or log capture verifies events/spans are emitted.
2. **E2E tests** — Provider replay still works; no tracing overhead breaks fixtures.
3. **Live tmux tests** — Enable `RUNIE_LOG=debug` and submit a prompt; verify provider spans appear.

## Tests

### Unit tests
- Span/event emission for key provider functions.

### E2E tests
- Replay with `tracing` enabled.

### Live tmux tests
- Run with debug logging and inspect provider trace.
