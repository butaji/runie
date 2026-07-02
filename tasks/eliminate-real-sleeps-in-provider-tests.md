# Eliminate real sleeps in provider tests

## Status

`todo`

## Description

Provider tests use real delays (`RUNIE_MOCK_DELAY=1`, 10 ms default, wall-clock timeout assertions). Replace with deterministic/mock time or set delays to 0.

## Acceptance criteria

1. **Unit tests** — No wall-clock sleeps; tests run deterministically.
2. **E2E tests** — Provider replay still validates ordering and cancellation.
3. **Live tmux tests** — Not applicable; test-only task.

## Tests

### Unit tests
- Mock time advances delay without real sleep.

### E2E tests
- Replay fixture with timeout/cancellation.

### Live tmux tests
- N/A.
