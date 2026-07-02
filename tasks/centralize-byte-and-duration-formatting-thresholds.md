# Centralize byte and duration formatting thresholds

## Status

`todo`

## Description

`tool/format.rs` uses raw literals for byte thresholds (`1000`, `1_000_000`, `1_000_000_000`) and duration thresholds (`60.0`). Centralize these as named constants.

## Acceptance criteria

1. **Unit tests** — Formatting output matches old behavior for representative values.
2. **E2E tests** — Tool output display in replay is unchanged.
3. **Live tmux tests** — Run tools that produce byte/time output in tmux and verify formatting.

## Tests

### Unit tests
- Byte and duration formatting at boundary values.

### E2E tests
- Replay fixture with tool result sizes/times.

### Live tmux tests
- Run a bash command and view the output.
