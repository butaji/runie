# Add parameterized tests with `test-case`

## Status

`todo`

## Description

Many parser/provider tests are repeated for similar inputs. Use `test-case` to reduce boilerplate and improve coverage.

## Acceptance criteria

1. **Unit tests** — Representative parser/provider tests use `test-case` and cover more inputs.
2. **E2E tests** — Existing replay tests still pass.
3. **Live tmux tests** — Not applicable; test-only task.

## Tests

### Unit tests
- Parameterized status-code classification, SSE parsing, shim formats.

### E2E tests
- Existing replay fixtures.

### Live tmux tests
- N/A.
