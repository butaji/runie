# Fix env lock isolation and remove duplicates

## Status

`todo`

## Description

There are three `ENV_LOCK` definitions. Consolidate on `runie_testing::ENV_LOCK` and ensure every `set_var`/`remove_var` acquires it. Fix `temp_home()` so `HOME` is isolated per test.

## Acceptance criteria

1. **Unit tests** — Concurrent env-mutating tests pass reliably; `temp_home()` returns isolated dirs.
2. **E2E tests** — Tests that rely on `HOME`/`RUNIE_*` env vars pass in any order.
3. **Live tmux tests** — Not applicable; test-only task.

## Tests

### Unit tests
- `ENV_LOCK` guards all env mutations.
- `temp_home()` isolation across calls.

### E2E tests
- Full test suite passes with randomized order.

### Live tmux tests
- N/A.
