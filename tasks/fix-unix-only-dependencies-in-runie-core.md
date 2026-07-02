# Fix Unix-only dependencies in `runie-core`

## Status

`todo`

## Description

`async-trait`, `derive_builder`, `ractor`, `parking_lot`, `uuid` are incorrectly placed under `target.'cfg(unix)'`. Move them to regular `[dependencies]` and remove duplicate `tempfile`.

## Acceptance criteria

1. **Unit tests** — `runie-core` compiles on non-Unix targets (at least `cargo check --target`).
2. **E2E tests** — Smoke tests pass on the host.
3. **Live tmux tests** — Run the host build in tmux.

## Tests

### Unit tests
- Target compilation check.

### E2E tests
- Host smoke tests.

### Live tmux tests
- Launch host binary.
