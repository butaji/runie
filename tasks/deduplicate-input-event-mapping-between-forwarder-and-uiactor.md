# Deduplicate input-event mapping between forwarder and `UiActor`

## Status

`done`

## Description

`input_forwarder_task` and `handle_input_event` both map keys to `InputMsg`. There should be one canonical converter.

## Acceptance criteria

1. **Unit tests** — All key sequences map to the same `InputMsg` regardless of entry point.
2. **E2E tests** — Input events from both paths produce identical state changes.
3. **Live tmux tests** — Typing, pasting, and special keys work normally in tmux.

## Tests

### Unit tests
- Key-to-`InputMsg` mapping table.

### E2E tests
- Forwarder and `UiActor` paths produce identical input state.

### Live tmux tests
- Type, paste, navigate history, and submit in tmux.
