# Route TUI autocomplete through `InputActor` events

## Status

`todo`

## Description

`UiActor` writes `input_mut()` directly for `@` and `/` autocomplete triggers. Route these through `InputActor` as events (`Event::OpenAtFilePicker`, `Event::OpenCommandPalette`).

## Acceptance criteria

1. **Unit tests** — Autocomplete triggers emit the correct events and update input state through `InputActor`.
2. **E2E tests** — Key sequences for file/command autocomplete still open the right dialogs.
3. **Live tmux tests** — Press `@` and `/` in tmux and confirm the pickers open.

## Tests

### Unit tests
- `@`/`/` key handling emits expected events.

### E2E tests
- Feed crossterm key events into `UiActor` and assert dialog opens.

### Live tmux tests
- In tmux, type `@` to open file picker and `/` to open command palette.

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** `InputActor` owns input state; autocomplete routes through it.
- [ ] **Trigger events:** `OpenAtFilePicker`, `OpenCommandPalette` trigger autocomplete.
- [ ] **Observer events:** `InputChanged`, `AutocompleteOpened` notify observers.
- [ ] **No direct mutations:** `UiActor` must not directly mutate `InputActor` state; emit events instead.
- [ ] **No new mirrors:** Input state is authoritative in `InputActor`; no duplicates.
- [ ] **Async work observed:** N/A (synchronous autocomplete handling).
