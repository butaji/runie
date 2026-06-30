# Simplify `word_wrap` to a single pass

**Status**: done
**Milestone**: R7

**Note**: Verified 2026-06-29 — single-pass logic avoids re-wrapping already-short lines; all word_wrap tests pass.
**Category**: Core / State
**Priority**: P2

**Depends on**: unify-core-and-tui-line-count-computation
**Blocks**: none

## Description

`crates/runie-core/src/layout.rs::word_wrap` wraps text twice: first to `first_width`, then re-wraps each line to either `first_width` or `rest_width`. Use `textwrap::Options` with `initial_indent`/`subsequent_indent` or a custom `WordSeparator` to honor both widths in one pass.

## Acceptance Criteria

- [ ] `word_wrap` produces the same output in a single pass.
- [ ] All existing tests pass.
- [ ] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [ ] `word_wrap_single_pass_matches_double_pass` — outputs equal for a grid of inputs.

## Files touched

- `crates/runie-core/src/layout.rs`

## Notes

- Keep ANSI escape handling intact.
