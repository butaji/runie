# Unify markdown block parsing and healing on `pulldown-cmark` events

**Status**: todo
**Milestone**: R6
**Category": Core / State
**Priority": P1

**Depends on**: unify-markdown-processing-around-pulldown-cmark
**Blocks**: replace-tui-markdown-block-layout-with-tui-markdown

## Description

`crates/runie-core/src/markdown/blocks.rs` re-injects inline markers (`**`, `*`, `~~`) into a text buffer so `parse_inline_spans` can re-parse them. `heal.rs` uses a char-level state machine to close unclosed inline syntax. Rewrite both to operate on a single `pulldown-cmark` event stream, storing inline styles directly from events.

## Acceptance Criteria

- [ ] Rewrite `BlockParser` to collect styled spans directly from `pulldown-cmark` events.
- [ ] Rewrite `heal_markdown` to use event-driven closing of unclosed fences/inline syntax.
- [ ] Delete the char-level state machine and marker re-injection.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `block_parser_round_trip` — markdown parses to styled spans and back.
- [ ] `heal_unclosed_inline` — unclosed `**` is closed correctly.
- [ ] `heal_unclosed_fence` — unclosed code fence is closed correctly.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `provider_replay_markdown_heals` — a provider stream with malformed markdown renders correctly.

## Files touched

- `crates/runie-core/src/markdown/blocks.rs`
- `crates/runie-core/src/markdown/inline.rs`
- `crates/runie-core/src/markdown/heal.rs`
- `crates/runie-core/src/markdown/mod.rs`

## Notes

- This unblocks `replace-tui-markdown-block-layout-with-tui-markdown.md`.
