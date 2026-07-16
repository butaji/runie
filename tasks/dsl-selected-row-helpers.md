# DSL selected row helpers

## Objective

Add DSL helpers for asserting which row is selected in dialogs, pickers, and
lists.

## Why this matters

Tests currently capture the pane and manually check for `▸` or parse a
`selected_line` helper. A DSL helper makes these assertions explicit and robust.

## Proposed helpers

Implemented in `src/app_test.rs`:

```rust
app.expect_selected_row("mock/echo").await?;
```

Implementation captures the pane and asserts the regex `▸[^\n]*pattern`
matches. `expect_selection_glyphs()` was not needed; `expect_selected_row`
covers the common case.

## Files that will benefit

- `tests/dialog_navigation.rs:14`
- `tests/command_palette_navigation.rs:71`
- `tests/model_switching.rs:37,56`

## Dependencies

- `black_box_replay_dsl`

## Acceptance checklist

- [x] `expect_selected_row(pattern)` helper exists and works for command
      palette, model picker, and permission dialog.
- [x] At least the three files above are converted.
- [x] No manual `selected_line` parsing remains in tests.
