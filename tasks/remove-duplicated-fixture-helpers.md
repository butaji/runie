# Remove duplicated fixture and temp-home helpers

## Objective

Eliminate local `fixture_path()` and `temp_home()` helpers in test files and
replace them with DSL helpers.

## Why this matters

`tests/cli_replay.rs` and `tests/tui_replay_conversations.rs` define their
own fixture-path and temp-home helpers instead of using the DSL. This duplicates
logic and makes it harder to change the fixture layout or isolation strategy.

## Required DSL helpers

Implemented:

- `fixture_path!("openai/opencode_go_...sse")` macro in `src/fixtures.rs`.
- `test_cli()` / `test_tui()` builders create temp homes automatically in
  `src/cli.rs` and `src/tui.rs`.

## Files to update

- `tests/cli_replay.rs`
- `tests/tui_replay_conversations.rs`
- Any other test file with local helpers

## Dependencies

- `black_box_replay_dsl`

## Acceptance checklist

- [x] No local `fixture_path()` or `temp_home()` helpers remain in
      `tests/cli_replay.rs` or `tests/tui_replay_conversations.rs`.
- [x] These tests use the DSL helpers.
- [x] Tests pass.
