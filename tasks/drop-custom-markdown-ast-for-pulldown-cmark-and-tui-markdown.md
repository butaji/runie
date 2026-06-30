# Drop custom markdown AST and render through pulldown-cmark/tui-markdown

## Status

`todo`

## Context

Runie maintains a hand-rolled markdown AST (`MdInline`, `CodeBlock`, `BlockParser`) in `crates/runie-core/src/markdown/{blocks,inline,heal,mod}.rs` plus a TUI re-renderer in `crates/runie-tui/src/markdown_render.rs`. This duplicates work already done by `pulldown-cmark` (already a dep) and `tui-markdown` (already a dep). The custom AST is ~700–900 lines and is a recurring source of wrap/heal/line-count bugs.

## Goal

Remove the custom markdown AST and render markdown directly: parse with `pulldown-cmark`, render blocks via `tui-markdown`, and apply only Runie-specific overlays (timestamps, glyphs, bubble margins, code-block headers) on top.

## Acceptance Criteria

- [ ] Delete `crates/runie-core/src/markdown/blocks.rs`, `inline.rs`, `heal.rs`, or reduce them to thin helpers.
- [ ] `crates/runie-tui/src/markdown_render.rs` renders `tui_markdown::Text`/`Line`/`Span` instead of converting to `MdSpan`.
- [ ] Line-count math in `crates/runie-core/src/layout.rs` stays in sync with rendered output.
- [ ] All existing markdown snapshot tests pass without visual changes.
- [ ] Edge cases preserved: nested lists, blockquotes, code fences, inline styles, incomplete fences.

## Design Impact

No change to TUI element design or composition. Only the markdown parsing/rendering implementation changes; visible output must remain pixel-identical.

## Tests

- **Layer 1 — State/Logic:** Unit tests for pulldown-cmark event → `tui-markdown` output mapping.
- **Layer 2 — Event Handling:** Feed streaming markdown deltas and assert the correct `Event::MessageDelta` facts.
- **Layer 3 — Rendering:** `TestBackend` snapshot tests for code blocks, lists, blockquotes, inline styles, and incomplete fences must match existing snapshots.
- **Layer 4 — E2E:** Provider replay fixture streams markdown and produces the same rendered output.
- **Live tmux validation:** Start a turn that returns a code block, list, and blockquote; verify the rendered messages look identical to before.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
