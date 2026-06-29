# Replace hand-rolled TUI markdown block layout with `tui-markdown`

**Status**: todo
**Milestone**: R6
**Category": TUI / Rendering
**Priority": P0

**Depends on**: unify-markdown-processing-around-pulldown-cmark
**Blocks**: none

## Description

Agent messages parse markdown into a custom `CodeBlock` AST and hand-layout code headers, lists, blockquotes, and timestamps. `tui-markdown` is already a dependency but only used for inline spans. Use `tui-markdown` to produce styled `Text`/`Line`s and overlay timestamps, glyphs, and bubble margins.

## Acceptance Criteria

- [ ] Use `tui-markdown` to convert markdown to styled `Text`/`Line`s.
- [ ] Overlay timestamps, glyphs, and bubble margins on top.
- [ ] Delete the custom `CodeBlock`, list, blockquote, and code-header layout code.
- [ ] Preserve visual output for code blocks, lists, blockquotes, inline styles.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 3 — Rendering
- [ ] `code_block_renders_with_tui_markdown` — `TestBackend` buffer matches expected code block.
- [ ] `list_renders_with_tui_markdown` — list buffer matches expected output.
- [ ] `blockquote_renders_with_tui_markdown` — blockquote buffer matches expected output.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `provider_replay_message_renders` — a provider replay with markdown content renders correctly.

## Files touched

- `crates/runie-tui/src/message/mod.rs`
- `crates/runie-tui/src/message/support.rs`
- `crates/runie-tui/src/message/code.rs`
- `crates/runie-tui/src/markdown_render.rs`

## Notes

- This is the highest-impact TUI simplification after the input box and popup list.
- If `tui-markdown` cannot render some custom element, extend it or keep a tiny local wrapper.
