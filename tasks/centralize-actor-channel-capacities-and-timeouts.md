# Centralize actor channel capacities and timeouts

## Status

`todo`

## Description

Actor code contains unexplained literals for channel capacities (`32`, `1000`, `16`), shutdown timeout (`5`), debounce (`300`), and speed-window capacity (`1000`). Centralize these as named constants or config values.

## Acceptance criteria

1. **Unit tests** — Each actor module exposes named constants for capacities/timeouts; tests verify positive values.
2. **E2E tests** — Actor spawn/replay smoke tests pass unchanged.
3. **Live tmux tests** — Run a multi-turn session in tmux and verify no dropped events.

## Tests

### Unit tests
- Constants are non-zero and referenced by production code.

### E2E tests
- Leader bootstrap and shutdown replay works.

### Live tmux tests
- Queue several messages and confirm all turns complete.
