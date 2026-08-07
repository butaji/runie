# Step 04: Scrollback widget

**Status:** implemented; Grok typed-card parity remains active in p16/p21
**Depends on:** 03

## Goal
A scrollback widget: append lines, autoscroll on append, render in a given `Rect`.

## Changes
- `crates/runie-tui/src/widgets/scrollback.rs`:
  - `pub struct Scrollback { lines: Vec<Line>, autoscroll: bool, scroll_offset: usize }`.
  - `Line` struct: `kind: LineKind`, `text: String`.
  - `LineKind` enum: `User`, `Assistant`, `Tool`, `ToolResult`, `System`.
  - `pub fn append(&mut self, line: Line)`: appends; if autoscroll, sets scroll_offset to follow tail.
  - `pub fn render(&self, area: Rect, buf: &mut Buffer)`: walks lines from `scroll_offset` to end, applies styling per `LineKind`, wraps with `textwrap` or ratatui's `Line::from(...)`.
  - `pub fn clear(&mut self)`.

## Verification
- Unit test: append 100 lines, assert `scroll_offset` follows.
- Render test (TestBackend): render the scrollback and assert lines appear in order.

## Notes
- `textwrap` is already in `runie-core`'s workspace deps; reuse it.
