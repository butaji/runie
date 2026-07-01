# Extend markdown inline styles to lists and blockquotes

**Status**: done
**Milestone**: R7
**Category**: Core / State
**Priority**: P2

**Depends on**: unify-markdown-block-parsing-on-pulldown-cmark-events
**Blocks**: none

## Description

`crates/runie-core/src/markdown/blocks.rs` accumulates plain text only for `List` and `Blockquote` states. Extend `BlockParser` to emit styled inline spans for list items and blockquotes, then update layout and renderers.

## Acceptance Criteria

- [x] `List` state preserves inline styles (bold, italic, code, links).
- [x] `Blockquote` state preserves inline styles.
- [x] `layout.rs` line counts and TUI renderers honor them.
- [x] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [x] `list_item_with_inline_styles` — list item emits styled spans.

### Layer 3 — Rendering
- [x] `blockquote_renders_inline_styles` — TestBackend buffer shows styled text inside blockquote.

## Files touched

- `crates/runie-core/src/markdown/blocks.rs`
- `crates/runie-core/src/layout.rs`
- `crates/runie-tui/src/message/mod.rs`
- `crates/runie-tui/src/message/support.rs`
- `crates/runie-core/src/markdown/tests.rs`
- `crates/runie-tui/src/ui/messages/mod.rs`

## Notes

- This removes a deliberate limitation, not a `pulldown-cmark` limitation.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
