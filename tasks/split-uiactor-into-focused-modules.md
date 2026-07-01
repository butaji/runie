# Split `ui_actor.rs` into focused modules

## Status

`todo`

## Description

`crates/runie-tui/src/ui_actor.rs` is 794 lines and mixes event routing, input dispatch, effect dispatch, animation, autocomplete, and submit logic.

## Acceptance criteria

1. **Unit tests** — Split modules compile and existing unit tests pass.
2. **E2E tests** — Key/submit/autocomplete events still route correctly via `TestBackend`.
3. **Live run tests** — Run the TUI in tmux and verify input, effects, and rendering work after the split.

## Tests

### Unit tests
- Split modules compile and existing tests pass.

### E2E tests
- Key/submit/autocomplete events still route correctly.

### Live run tests
- Launch the app in tmux, type input, submit a message, and observe effects.
