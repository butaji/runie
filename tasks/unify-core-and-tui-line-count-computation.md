# Unify core and TUI line-count computation

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P2

**Depends on**: replace-custom-helpers-with-crates
**Blocks**: none

## Description

Line wrapping and line-count logic exist in both `runie-core` and `runie-tui`, producing inconsistent message heights and duplicated code. Unify the computation in one place, either a shared helper in `runie-core` or the `textwrap` crate used consistently.

## Changes Made

### Already Unified

The line count computation is already unified:

1. **`runie-core/src/layout.rs`** - Contains `word_wrap()` and `element_line_count()` functions
2. **`runie-tui/src/message/wrap.rs`** - Re-exports `word_wrap` from `runie_core::layout`
3. **`runie-tui/src/message/mod.rs`** - Uses `wrap_styled_spans()` which uses `word_wrap`

The `word_wrap` function in `runie-core` is the single source of truth for word wrapping, using `textwrap` crate.

### Key Design Points

- `runie_core::layout::word_wrap()` - Core word-wrapping using `textwrap`
- `runie_core::layout::element_line_count()` - Core element line counting
- `runie_tui::message::wrap::wrap_styled_spans()` - TUI-specific styled wrapping
- `runie_tui::message::mod.rs` - Re-exports `word_wrap` for TUI use

## Acceptance Criteria

- [x] Exactly one source of truth computes wrapped line counts. (`runie_core::layout`)
- [x] TUI message/diff views use the core helper or `textwrap`. (TUI re-exports from core)
- [x] Duplicate wrapping logic in TUI is deleted. (TUI only has styled span wrapping, which builds on core)
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `line_count_matches_textwrap` — core helper produces the same count as `textwrap` for a grid of inputs.
- [x] `wrapped_height_for_message` — message height calculation matches manual expectation.
- [x] `word_wrap_handles_wide_chars` — CJK and emoji are handled correctly.
- [x] `element_line_count_spacer_is_one` — spacer elements have correct height.
- [x] `user_message_line_count_matches_wide_viewport` — user messages have correct heights.

### Layer 3 — Rendering
- [x] TUI message rendering uses the unified `word_wrap` from core.

## Files touched

- No changes needed - already unified

## Notes

- The `textwrap` crate is used consistently for word wrapping.
- ANSI escape sequences are handled via `display_width::width` which counts display cells correctly.
- Core scroll math and TUI renderer share the exact same wrapping rules.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
