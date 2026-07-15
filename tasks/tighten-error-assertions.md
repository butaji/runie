# Tighten error state assertions

## Objective

Make `tests/error_state_rendering.rs` assertions specific and remove redundant
sequential-submit tests.

## Why this matters

Current patterns like `error|Error|Invalid|invalid|failed` match too many
strings and can hide real failures. `sequential_submits_no_corruption` and
`sequential_submits_work` also overlap significantly.

## Required changes

1. Merge `sequential_submits_no_corruption` and `sequential_submits_work` into a
   single test that verifies sequential submits produce distinct, non-corrupted
   responses.
2. Replace broad regex alternations with specific expected strings:
   - Invalid API key: `Invalid API key` or `Authentication failed`.
   - Rate limit: `Rate limit exceeded` or `429`.
   - Missing fixture: `Fixture not found` or similar exact CLI/TUI message.
3. Assert on exit codes for CLI tests.

## Files to update

- `tests/error_state_rendering.rs`
- `tasks/error_state_rendering.md`

## Dependencies

- `cli_replay_dsl`
- `tui_dsl_polling_waits`

## Acceptance checklist

- [ ] No broad `error|Error|Invalid|invalid|failed` patterns remain.
- [ ] Only one sequential-submit test exists.
- [ ] CLI tests assert specific exit codes.
- [ ] All error-state tests pass.
