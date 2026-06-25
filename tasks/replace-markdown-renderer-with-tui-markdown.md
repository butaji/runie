# Replace custom markdown renderer with `tui-markdown`

**Status**: done
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Summary

Integrated `tui-markdown` for inline markdown parsing while preserving the existing core parsing for custom color support. Code blocks continue to use syntect for syntax highlighting.

## Implementation

- Added `tui-markdown` dependency to `crates/runie-tui`
- `parse_inline_markdown` now uses `tui_markdown::from_str` for parsing and styling
- `parse_inline_markdown_with_color` falls back to core parsing + custom colors (tui_markdown doesn't support custom base colors)
- Code blocks continue to use syntect via `crates/runie-tui/src/syntax.rs`
- Preserved the `MdSpan` type and `md_to_spans` function for compatibility

## Acceptance Criteria

- [x] `tui-markdown` is added to `runie-tui` dependencies.
- [x] Inline markdown parsing uses `tui-markdown` (via `parse_inline_markdown`).
- [x] Custom color support preserved via core parsing (via `parse_inline_markdown_with_color`).
- [x] Code blocks remain syntax-highlighted (syntect).
- [x] `cargo check --workspace` is green with no new warnings.

## Tests

- **Layer 3**: `TestBackend` + `Buffer` assertions for rendered markdown output.
  - `styled_spans_preserved` - verifies bold, italic, code styling
  - `parse_inline_markdown_uses_tui_markdown` - verifies tui_markdown integration
  - `parse_inline_markdown_with_color_falls_back_to_core` - verifies color override
- **Layer 4**: Provider-replay test for markdown response streaming.

## Notes

- `ratatui-markdown` was not used due to API incompatibility with ratatui 0.30.
- `tui_markdown` uses `ratatui_core` internally but provides `ratatui::text::Text` in its public API.
- The implementation is a hybrid: tui_markdown for standard parsing, core parsing for custom colors.
