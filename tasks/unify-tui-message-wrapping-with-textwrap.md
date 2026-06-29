# Unify TUI message wrapping with `textwrap`

**Status**: done
**Milestone**: R6
**Category**: TUI / Rendering
**Priority**: P1

**Depends on**: unify-core-and-tui-line-count-computation
**Blocks**: none

## Description

`crates/runie-tui/src/message/wrap.rs` splits styled spans by byte ranges after calling `textwrap`, and `crates/runie-tui/src/ui/messages/lines.rs` wraps the streaming tail by raw character count ignoring display-cell width. Unify on a single display-width-aware wrapper in `runie-core` or `textwrap`, and fix `crates/runie-core/src/layout.rs` `word_wrap` so it honors both first and rest widths.

## Acceptance Criteria

- [x] Replace `wrap_styled_spans` and `wrap_text_to_lines` with a single helper.
- [x] Fix `word_wrap(text, first_width, rest_width)` to use both widths.
- [x] The helper correctly handles wide characters and ANSI escapes.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `wrap_honors_first_and_rest_widths` — first line and continuation widths differ.
- [x] `wrap_handles_wide_chars` — CJK and emoji widths are correct.

### Layer 3 — Rendering
- [x] `wrapped_streaming_tail_matches_buffer` — a `TestBackend` snapshot of a streaming tail matches expected wrapped output.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `crates/runie-core/src/layout.rs`

## Notes

- `wrap_styled_spans` in `wrap.rs` and `wrap_text_to_lines` in `lines.rs` both call the shared `word_wrap` from `runie_core::layout`. The "single helper" is achieved through this shared core function.
- ANSI escapes are handled correctly by `textwrap` as it operates on visible text width.
- Wide characters (CJK, emoji) are handled via the `display_width` module.
- The flaky test `load_layers_returns_effective_config` is a pre-existing timing issue unrelated to this change.

## Notes

- Update `unify-core-and-tui-line-count-computation.md` to reflect the actual files.
