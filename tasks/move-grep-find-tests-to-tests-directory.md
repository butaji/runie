# Move grep_find tests to tests directory

## Status

`todo`

## Context

`crates/runie-agent/src/grep_find.rs` is a production source file that contains only a `#[cfg(test)]` module. This clutters the src tree and prevents integration-test-style organization.

## Goal

Move the tests into `crates/runie-agent/src/tests/parser.rs` or an integration test under `crates/runie-agent/tests/`.

## Acceptance Criteria

- [ ] Remove `grep_find.rs` from `src/`.
- [ ] Move tests to a proper test module/file.
- [ ] Update `lib.rs` / `mod.rs` references.
- [ ] Ensure `cargo test` still discovers and runs the tests.

## Tests

- **Layer 1 — State/Logic:** Moved tests still pass and cover the same parser cases.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** N/A.
- **Live tmux validation:** N/A.
