# Collapse `Intent` into shared event payloads

## Status

`todo`

## Description

`Intent` duplicates many `Event` intent/control variants, and `Event::into_intent()` maps them manually. Share struct payloads (e.g., `RunLoadCommand { name }`) between `Event` and `Intent`, or derive `Intent` from `Event`.

## Acceptance criteria

1. **Unit tests** — `Event::into_intent()` is either derived or removed; every intent/control variant converts correctly.
2. **E2E tests** — Slash commands and palette commands still produce the right `Intent`.
3. **Live tmux tests** — Run `/load`, `/save`, and other slash commands in tmux and verify behavior.

## Tests

### Unit tests
- Conversion from `Event` to `Intent` for all shared variants.

### E2E tests
- Command palette and slash command events produce correct intents.

### Live tmux tests
- Open tmux, type `/load <name>`, and confirm the command runs.
