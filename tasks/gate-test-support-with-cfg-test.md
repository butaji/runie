# Gate test support with `#[cfg(test)]`

## Status

`todo`

## Description

`runie-core/src/tests/mod.rs` and `tests/support.rs` are compiled into non-test builds. Gate them with `#[cfg(test)]` and move shared helpers to `runie-testing` if needed.

## Acceptance criteria

1. **Unit tests** — Helpers are still available in tests; production builds no longer include them.
2. **E2E tests** — Smoke tests pass.
3. **Live tmux tests** — Launch the production binary and verify no test helpers are present.

## Tests

### Unit tests
- `cfg(test)` guards test helper modules.

### E2E tests
- Binary smoke test still runs.

### Live tmux tests
- Run the CLI in tmux.
