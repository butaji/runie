# Unify Rendering Pipeline

**Status**: todo
**Milestone**: R3
**Category**: TUI / Rendering
**Priority**: P1

**Depends on**: merge-runie-term-into-tui
**Blocks**: (none)

**Re-opened**: 2026-06-16 — `runie-core/src/ui/` still contains element/feed/transform logic tied to the view layer, and `runie-tui/src/ui/messages.rs` duplicates scroll/render concerns.

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

- [x] `runie-core` exposes a renderable message/element AST, not Ratatui
  widgets.
- [x] `runie-tui` is the only crate that imports `ratatui` for rendering.
- [x] The core test renderer (`ui/transform.rs`, `ui/posts.rs`, etc.) is
  deleted or folded into TUI `TestBackend` tests.
- [x] Scroll math and line counts are computed from the same AST the TUI
  renders.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 3 — Rendering
- [x] `test_formatted_labels_short_names` — renders a tool flow and verifies
  ✓ / duration / "Turn completed" appear.
- [x] `test_list_files_full_tool_flow_sequence` — renders full tool flow and
  verifies list_files / ✓ / → / turn_complete.

## Files touched

- `crates/runie-core/src/ui/*.rs`
- `crates/runie-tui/src/ui/messages.rs`
- `crates/runie-tui/src/message/mod.rs`
- `crates/runie-tui/src/tests/*`
- `crates/runie-core/src/tests/*` (render-related tests)

## Notes

This unifies the two markdown parsers as a side effect: core decomposes
blocks once, TUI styles them.
