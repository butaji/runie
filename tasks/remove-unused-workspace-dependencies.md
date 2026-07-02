# Remove unused workspace dependencies

## Status

`todo`

## Description

`tracing-appender` and possibly other workspace dependencies are unused. Remove them from the workspace `Cargo.toml`.

## Acceptance criteria

1. **Unit tests** — Workspace builds with no unused workspace-level deps.
2. **E2E tests** — Smoke tests pass.
3. **Live tmux tests** — Not applicable.

## Tests

### Unit tests
- Grep confirms no crate uses the removed dep.

### E2E tests
- Full workspace build.

### Live tmux tests
- N/A.
