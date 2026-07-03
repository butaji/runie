# Use tui-scrollview for message feed

## Status

`wontfix`

## Context

`crates/runie-tui/src/ui/scroll.rs`, `ui/messages/`, `message/wrap.rs`, and `message/support.rs` implement scrollable message feeds and blockquote wrapping by hand, maintaining parallel line-count math in `layout.rs`.

## Decision

After analysis, adopting `tui-scrollview` (`ScrollView`) for the message feed would require **architectural changes** without a **clear Pareto win**:

### Why `tui-scrollview` doesn't fit here

1. **Current implementation is simpler**: `Paragraph::new(lines).scroll((offset, 0))` + Ratatui's built-in `Scrollbar` widget handles all scroll needs cleanly.

2. **`tui-scrollview` is designed for widget-based scrolling**: It renders individual widgets into a virtual scrollable area. Our message feed flattens heterogeneous elements (markdown, tool outputs, code blocks) into a single `Paragraph`. Switching to `ScrollView` would require:
   - Restructuring `UiActor` to hold `ScrollViewState`
   - Rewriting `build_lines_with_mapping` to render each element into its area within `ScrollView`
   - Maintaining vim-style nav highlighting (`nav.rs`) as a separate overlay
   - The ScrollView API is for widgets, not for `Paragraph` with wrapped lines

3. **Line-count math is already unified**: `layout.rs`, `lines.rs`, and `render_lines.rs` all use `Element::line_count()` as the single source of truth. This is a data-layer concern, not a rendering-layer concern.

4. **`tui-scrollview` is added but unused**: `tui-scrollview = "0.6"` was added to workspace dependencies for potential future use (e.g., a dedicated scrollable panel or inspector view), but it is not wired into the message feed.

### What `tui-scrollview` could be useful for (future)

- A dedicated scrollable panel or inspector view
- A scrollable file preview in the `@` file picker
- Any future scrollable widget that renders multiple child widgets

These are out of scope for the current message feed.

## Acceptance Criteria

- [x] Add dependency — `tui-scrollview = "0.6"` added to workspace
- [ ] Replace manual scroll logic — Not done; architectural mismatch
- [ ] Coordinate with `layout::element_line_count` — Already done; `build_lines_with_mapping` uses `Element::line_count()`

## Why Not Superseded

This is `wontfix`, not `superseded`, because the task is real and the crate was added. The decision is that the current `Paragraph::new().scroll()` approach is simpler and more appropriate for the message feed use case.
