# Consolidate message validation in one module

## Status

`todo`

## Description

`proto/message/mod.rs` and `sanitize.rs` both validate `ChatMessage` sequences. Consolidate all validation in one place.

## Acceptance criteria

1. **Unit tests** — All validation edge cases are covered in the single module.
2. **E2E tests** — Replay turns with invalid messages are rejected/sanitized the same way.
3. **Live tmux tests** — Paste unusual message content in tmux and confirm no validation panics.

## Tests

### Unit tests
- Empty messages, missing roles, malformed tool calls, etc.

### E2E tests
- Mock-provider turn exercises sanitization path.

### Live tmux tests
- Paste multi-line and special characters into the composer.
