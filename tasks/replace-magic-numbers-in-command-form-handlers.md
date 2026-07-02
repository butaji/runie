# Replace magic numbers in command form handlers

## Status

`todo`

## Description

`commands/dsl/handlers/session/mod.rs` and `run.rs` contain hardcoded form placeholders (`"2000"`, `"0"`) and fallback indices. Replace with named constants.

## Acceptance criteria

1. **Unit tests** — Form default values and fallback indices are named constants.
2. **E2E tests** — Compact/fork/session commands work in replay.
3. **Live tmux tests** — Run `/compact`, `/fork`, `/save`, `/load` in tmux.

## Tests

### Unit tests
- Constants for compact keep-tokens and fork fallback index.

### E2E tests
- Replay fixtures for compact/fork/save/load.

### Live tmux tests
- Use slash commands interactively.
