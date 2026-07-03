# Simplify `word_wrap` to a single pass

**Status**: done
**Note**: Verified 2026-06-29 — `word_wrap` in `layout.rs` uses single-pass algorithm with `unicode-width`.
**Milestone**: R7

**Note**: Verified 2026-06-29 — single-pass logic avoids re-wrapping already-short lines; all word_wrap tests pass.
**Category**: Core / State
**Priority**: P2

**Depends on**: unify-core-and-tui-line-count-computation
**Blocks**: none

## Description

`crates/runie-core/src/layout.rs::word_wrap` wraps text twice: first to `first_width`, then re-wraps each line to either `first_width` or `rest_width`. Use `textwrap::Options` with `initial_indent`/`subsequent_indent` or a custom `WordSeparator` to honor both widths in one pass.

## Acceptance Criteria

- [x] `word_wrap` produces the same output in a single pass.
- [x] All existing tests pass.
- [x] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [x] `word_wrap_single_pass_matches_double_pass` — outputs equal for a grid of inputs.

## Files touched

- `crates/runie-core/src/layout.rs`

## Notes

- Keep ANSI escape handling intact.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
