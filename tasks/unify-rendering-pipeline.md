# Unify Rendering Pipeline

**Status**: todo
**Milestone**: R3
**Category**: TUI / Rendering
**Priority**: P1

**Depends on**: `merge-runie-term-into-tui`
**Blocks**: (none)

## Description

`runie-core/src/ui/` contains element/feed/transform logic that is used for
scroll math and tested with a core-specific test renderer.
`runie-tui/src/ui/messages.rs` and `runie-tui/src/message/mod.rs` contain
parallel rendering logic for Ratatui. The two pipelines must stay in sync
manually.

Core should produce a renderable AST; `runie-tui` should be the sole
renderer. Layer-3 tests should use Ratatui `TestBackend` rather than a
separate core test renderer.

## Acceptance Criteria

- [ ] `runie-core` exposes a renderable message/element AST, not Ratatui
  widgets.
- [ ] `runie-tui` is the only crate that imports `ratatui` for rendering.
- [ ] The core test renderer (`ui/transform.rs`, `ui/posts.rs`, etc.) is
  deleted or folded into TUI `TestBackend` tests.
- [ ] Scroll math and line counts are computed from the same AST the TUI
  renders.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 3 — Rendering
- [ ] `message_list_renders_with_test_backend` — a message list renders
  identically (or acceptably so) to the old core test renderer output.
- [ ] `line_counts_match_rendered_lines` — core line counts match the lines
  produced by the TUI renderer for the same AST.

## Files touched

- `crates/runie-core/src/ui/*.rs`
- `crates/runie-tui/src/ui/messages.rs`
- `crates/runie-tui/src/message/mod.rs`
- `crates/runie-tui/src/tests/*`
- `crates/runie-core/src/tests/*` (render-related tests)

## Notes

This unifies the two markdown parsers as a side effect: core decomposes
blocks once, TUI styles them.
