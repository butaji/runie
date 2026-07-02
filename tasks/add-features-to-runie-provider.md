# Add features to `runie-provider`

## Status

`todo`

## Description

`runie-provider` always builds all providers. Add features such as `openai` and `mock` so consumers can compile only what they need.

## Acceptance criteria

1. **Unit tests** — Feature matrix compiles; default includes needed providers.
2. **E2E tests** — Smoke tests pass with default features.
3. **Live tmux tests** — Build and run with only the OpenAI provider feature.

## Tests

### Unit tests
- `--no-default-features` and feature combinations compile.

### E2E tests
- Default feature replay passes.

### Live tmux tests
- Run a build with `mock` disabled.
