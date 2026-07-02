# Centralize UI strings module

## Status

`todo`

## Description

User-facing strings are scattered across handlers, update helpers, dialog builders, and tool code. Create `runie-core::ui_strings` (or similar) and move copy there.

## Acceptance criteria

1. **Unit tests** — All user-facing strings are referenced from the centralized module.
2. **E2E tests** — Help, usage, error, and placeholder messages still appear correctly.
3. **Live tmux tests** — Open help panel, run commands, and verify strings.

## Tests

### Unit tests
- No raw user-facing strings remain outside `ui_strings` (assert via code search).

### E2E tests
- Command handlers return expected strings.

### Live tmux tests
- Run `/help`, `/save`, `/compact` and read messages.
