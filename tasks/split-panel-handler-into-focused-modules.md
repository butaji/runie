# Split `panel_handler.rs` into focused modules

## Status

`todo`

## Description

`crates/runie-core/src/update/dialog/panel_handler.rs` is 586 lines and mixes navigation, activation, form handling, and settings toggles.

## Acceptance criteria

1. **Unit tests** — Navigation, activation, and form modules compile and pass focused unit tests.
2. **E2E tests** — Panel navigation, activation, and form submit events still work.
3. **Live run tests** — Open a dialog in tmux and exercise navigation, selection, and form submission.

## Tests

### Unit tests
- Split modules compile and tests pass.

### E2E tests
- Panel navigation, activation, and form submit events still work.

### Live run tests
- Open the command palette or a settings dialog in tmux and navigate/submit.
