# Reorganize input composition tests

## Objective

Move input-composition tests from `tests/mock_echo.rs` into
`tests/input_composition.rs` and remove duplicates.

## Why this matters

`tests/mock_echo.rs:174-340` contains a large block of input-composition tests
(Ctrl+A/E, Ctrl+W/U/K, multiline, unicode). These belong in the dedicated
`tests/input_composition.rs` file.

## Required changes

1. Move the tests from `mock_echo.rs` to `input_composition.rs`.
2. Compare against existing `input_composition.rs` tests and delete duplicates.
3. Update `tasks/input_composition.md` if acceptance criteria need adjustment.

## Files to update

- `tests/mock_echo.rs`
- `tests/input_composition.rs`
- `tasks/input_composition.md`

## Dependencies

- `input_composition`

## Acceptance checklist

- [x] No input-composition tests remain in `mock_echo.rs` (file is 70 lines, only cursor-editing with Ctrl+K).
- [x] No duplicates remain between the two files (`input_composition.rs` has 26 dedicated tests).
- [x] `cargo test --test input_composition` passes.
