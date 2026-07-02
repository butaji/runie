# Update build.rs comment on magic-number guardrail

## Status

**done**

## Description

The header comment in `runie-core/build.rs` says the guardrail catches literals `>= 10`, but the implementation catches `>= 1000`. Update the comment to match the code.

## Acceptance criteria

1. **Unit tests** — N/A; doc task.
2. **E2E tests** — N/A; doc task.
3. **Live tmux tests** — N/A; doc task.

## Tests

### Unit tests
- N/A.

### E2E tests
- N/A.

### Live tmux tests
- N/A.
