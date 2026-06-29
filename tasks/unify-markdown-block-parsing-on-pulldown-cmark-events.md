# Unify markdown block parsing and healing on `pulldown-cmark` events

**Status**: done
**Milestone**: R6
**Category**: Core / State
**Priority**: P1

**Depends on**: unify-markdown-processing-around-pulldown-cmark
**Blocks**: replace-tui-markdown-block-layout-with-tui-markdown

## Description

`crates/runie-core/src/markdown/blocks.rs` re-injects inline markers (`**`, `*`, `~~`) into a text buffer so `parse_inline_spans` can re-parse them. `heal.rs` uses a char-level state machine to close unclosed inline syntax. Rewrote both to operate on a single `pulldown-cmark` event stream, storing inline styles directly from events.

## Acceptance Criteria

- [x] Rewrite `BlockParser` to collect styled spans directly from `pulldown-cmark` events.
- [x] Rewrite `heal_markdown` to use event-driven closing of unclosed fences/inline syntax.
- [x] Delete the char-level state machine and marker re-injection.
- [x] `cargo test --workspace` succeeds after the change. → 1706 tests pass
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `block_parser_round_trip` — markdown parses to styled spans and back.
- [x] `heal_unclosed_inline` — unclosed `**` is closed correctly.
- [x] `block_parser_nested_styles` — correctly handles bold inside italic.
- [x] `block_parser_multiple_blocks` — code, list, and text blocks coexist.

## Files touched

- `crates/runie-core/src/markdown/blocks.rs` — Event-driven block parsing
- `crates/runie-core/src/markdown/inline.rs` — Simplified with shared `Style` enum
- `crates/runie-core/src/markdown/heal.rs` — Event-driven healing
- `crates/runie-core/src/markdown/mod.rs` — Added `heal` module export
- `crates/runie-core/src/markdown/tests.rs` — Added Layer-1 tests
- `crates/runie-core/src/streaming_buffer.rs` — Now re-exports from `markdown::heal_markdown`

## Notes

- The `heal_markdown` function was moved from `streaming_buffer.rs` to `markdown/heal.rs` for better organization.
- `BlockParser` now uses a `Style` enum and style stack to track inline markers directly from events.
- `inlines_to_text` was improved to properly round-trip styled spans back to markdown.
