# Audit mock provider delay constants

## Status

**done**

## Description

`runie-provider/src/lib.rs:143` uses bare `5` and `10` millisecond delays for `MockProvider::with_delay`. These are test-only but should be named constants.

## Acceptance criteria

1. **Unit tests** — `MOCK_DELAY_MIN_MS` and `MOCK_DELAY_MAX_MS` exist and are used.
2. **E2E tests** — Mock provider tests still pass.
3. **Live tmux tests** — Not applicable; test code.

## Tests

### Unit tests
- Constants are used by mock provider.

### E2E tests
- Mock provider replay passes.

### Live tmux tests
- N/A.
