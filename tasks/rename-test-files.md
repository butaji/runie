# Rename misleading test files

## Objective

Rename test files so their names accurately reflect whether they use mock or replay fixtures.

## Current state

- `tests/tui_replay_conversations.rs` now uses recorded SSE replay fixtures, so its name is accurate.
- `tests/cli_replay.rs` contains only CLI replay tests; the name is broader than the content.
- `tests/replay_dsl_smoke.rs` is a DSL smoke-test file and its name is accurate.

## Required change

Rename `tests/replay_blackbox.rs` to `tests/cli_replay.rs`.

## Implementation

1. `git mv tests/replay_blackbox.rs tests/cli_replay.rs`
2. Update `tests/cli_replay.rs` module doc comment to reflect new name.
3. Update any task docs that reference `tests/replay_blackbox.rs`:
   - `tasks/black-box-replay-testing.md`
   - `tasks/cli_replay_conversations.md`
   - `tasks/recorded-trace-coverage.md`
   - `tasks/improve-test-execution-speed.md`
   - `tasks/wire-anthropic-minimax-remaining-fixtures.md`
   - `tasks/wire-anthropic-qwen-remaining-fixtures.md`
   - `tasks/wire-error-sse-fixtures-to-cli-replay-tests.md`
   - `tasks/wire-openai-deepseek-remaining-multiturn-fixtures.md`
   - `tasks/wire-openai-glm-remaining-fixtures.md`
   - `tasks/wire-openai-kimi-remaining-fixtures.md`
   - `tasks/wire-openai-mimo-remaining-fixtures.md`
4. Verify `cargo test --test cli_replay` discovers and runs all tests.
5. Verify no other file imports or references `cli_replay`.

## Dependencies

- `cli_replay_conversations` (done)
- `tui_replay_conversations` (done)

## Acceptance checklist

- [x] `tests/replay_blackbox.rs` is renamed to `tests/cli_replay.rs`.
- [x] Every task doc referencing the old name is updated.
- [x] `cargo test --test cli_replay` passes.
- [x] `cargo test` still discovers all tests.
