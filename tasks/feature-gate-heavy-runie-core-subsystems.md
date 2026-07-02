# Feature-gate heavy `runie-core` subsystems

## Status

`todo`

## Description

`runie-core` compiles ~80 dependencies unconditionally. Add feature flags for MCP, keyring, git status, file watching, clipboard, markdown YAML, and model catalog YAML.

## Acceptance criteria

1. **Unit tests** — Each feature compiles independently; default build size is reduced.
2. **E2E tests** — Smoke tests pass with default features.
3. **Live tmux tests** — Build with minimal features and run tmux; core flows work.

## Tests

### Unit tests
- Feature matrix compiles.

### E2E tests
- Default feature smoke tests.

### Live tmux tests
- Run stripped-down build in tmux.
