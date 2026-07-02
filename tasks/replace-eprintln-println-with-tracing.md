# Replace `eprintln!`/`println!` with tracing

## Status

`todo`

## Description

Production code (`subagents/mod.rs`) and tests use `println!`/`eprintln!`. Replace with `tracing::debug!` or assertions; configure test subscriber.

## Acceptance criteria

1. **Unit tests** — No `println!`/`eprintln!` in production code; tests use `tracing` or assertions.
2. **E2E tests** — Output is unchanged when tracing is configured.
3. **Live tmux tests** — Run the TUI and confirm no stray stdout/stderr corrupts the terminal.

## Tests

### Unit tests
- Grep check for `println!`/`eprintln!` in production.

### E2E tests
- Smoke tests with tracing subscriber.

### Live tmux tests
- Launch TUI and check terminal integrity.
