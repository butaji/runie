# Break `runie-testing` dev-dependency cycle

## Status

`todo`

## Description

`runie-testing` depends on `runie-agent`/`runie-provider`, which dev-depend on `runie-testing`. Move provider/agent-specific mocks out of `runie-testing` or make `runie-testing` dev-dependency-only.

## Acceptance criteria

1. **Unit tests** — Dependency graph has no cycles; mocks live in the crates they test or in a dedicated support crate.
2. **E2E tests** — Replay tests still compile and pass.
3. **Live tmux tests** — Not applicable.

## Tests

### Unit tests
- `cargo metadata` shows no cycles.

### E2E tests
- Replay fixtures compile.

### Live tmux tests
- N/A.
