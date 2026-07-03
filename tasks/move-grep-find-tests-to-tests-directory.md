# Move grep_find tests to tests directory

**Status**: done
**Milestone**: R7
**Category**: Build / CI / Tests
**Priority**: P3

## Context

`crates/runie-agent/src/grep_find.rs` was a production source file that contained only a `#[cfg(test)]` module. This cluttered the src tree.

## Fix Applied

- Moved tests from `crates/runie-agent/src/grep_find.rs` to `crates/runie-agent/src/tests/parser.rs`
- Deleted `grep_find.rs`
- Removed `mod grep_find;` from `crates/runie-agent/src/lib.rs`

## Acceptance Criteria

- [x] Remove `grep_find.rs` from `src/`.
- [x] Move tests to a proper test module/file.
- [x] Update `lib.rs` / `mod.rs` references.
- [x] Ensure `cargo test` still discovers and runs the tests.

## Validation

- `cargo test --workspace`: all tests pass
- `cargo test --package runie-agent parse_grep`: passes
- `cargo test --package runie-agent parse_find`: passes
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
