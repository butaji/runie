# Split `panel_handler.rs` into focused modules

## Status

`todo`

## Description

`crates/runie-core/src/update/dialog/panel_handler.rs` is 586 lines and mixes navigation, activation, form handling, and settings toggles.

## Acceptance criteria

- Split into `navigation.rs`, `activation.rs`, `form.rs`.
- No module exceeds 500 lines.

## Tests

### Layer 2 — Event Handling
- Panel navigation, activation, and form submit events still work.
