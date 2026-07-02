# Derive `ChatMessageBuilder` with `derive_builder`

## Status

`todo`

## Description

`ChatMessageBuilder` is hand-written even though `derive_builder` is already a dependency. Derive it from `ChatMessage` to remove ~100 lines and prevent drift.

## Acceptance criteria

1. **Unit tests** — Derived builder produces the same `ChatMessage` instances as the hand-written one.
2. **E2E tests** — Message construction in replay turns works unchanged.
3. **Live tmux tests** — Send messages in tmux and verify they appear correctly.

## Tests

### Unit tests
- Builder defaults, field setters, and `build()` match existing behavior.

### E2E tests
- A mock-provider turn builds and sends messages correctly.

### Live tmux tests
- Type and submit a user message in tmux.
