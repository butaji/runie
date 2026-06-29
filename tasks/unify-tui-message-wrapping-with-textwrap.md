# Unify TUI message wrapping with `textwrap`

**Status**: todo
**Milestone**: R6
**Category**: TUI / Rendering
**Priority": P1

**Depends on**: unify-core-and-tui-line-count-computation
**Blocks**: none

## Description

`crates/runie-tui/src/message/wrap.rs` splits styled spans by byte ranges after calling `textwrap`, and `crates/runie-tui/src/ui/messages/lines.rs` wraps the streaming tail by raw character count ignoring display-cell width. Unify on a single display-width-aware wrapper in `runie-core` or `textwrap`, and fix `crates/runie-core/src/layout.rs` `word_wrap` so it honors both first and rest widths.

## Acceptance Criteria

- [ ] Replace `wrap_styled_spans` and `wrap_text_to_lines` with a single helper.
- [ ] Fix `word_wrap(text, first_width, rest_width)` to use both widths.
- [ ] The helper correctly handles wide characters and ANSI escapes.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `wrap_honors_first_and_rest_widths` — first line and continuation widths differ.
- [ ] `wrap_handles_wide_chars` — CJK and emoji widths are correct.

### Layer 3 — Rendering
- [ ] `wrapped_streaming_tail_matches_buffer` — a `TestBackend` snapshot of a streaming tail matches expected wrapped output.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-tui/src/message/wrap.rs`
- `crates/runie-tui/src/ui/messages/lines.rs`
- `crates/runie-core/src/layout.rs`

## Notes

- Update `unify-core-and-tui-line-count-computation.md` to reflect the actual files.
