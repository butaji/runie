# Use shell-words in harness skills

## Status

`done`

## Context

`crates/runie-core/src/harness_skills/verification_loop.rs:56` and `crates/runie-core/src/harness_skills/startup_context.rs:48` split commands with `command.split_whitespace().collect()`, which breaks quoted arguments. `shell-words` is already a workspace dependency.

## Goal

Use `shell_words::split` in harness skills for correct quoted-arg handling.

## Acceptance Criteria

- [x] Replace `split_whitespace()` with `shell_words::split`.
- [x] Handle parse errors gracefully.
- [x] Tests cover quoted paths and arguments with spaces.

## Implementation

Changed both `verification_loop.rs` and `startup_context.rs` to use `shell_words::split()` instead of `split_whitespace()`. Parse errors are handled by returning early (empty Vec or None).

## Tests Added

- `run_verification_simple_command` — basic command execution
- `run_verification_quoted_args` — single-quoted args with spaces
- `run_verification_double_quoted_args` — double-quoted args with spaces
- `run_verification_empty_command` — empty command returns None
- `run_verification_complex_args` — multiple quoted args
- `run_cmd_simple` — basic command execution
- `run_cmd_quoted_args` — single-quoted args with spaces
- `run_cmd_double_quoted_args` — double-quoted args with spaces
- `run_cmd_empty` — empty command returns empty
- `run_cmd_complex_args` — printf with multiple quoted args
- `run_cmd_with_escaped_chars` — escaped characters

All 44 harness_skills tests pass.

## Files Changed

- `crates/runie-core/src/harness_skills/verification_loop.rs`
- `crates/runie-core/src/harness_skills/startup_context.rs`
