# Replace custom markdown renderer with `tui-markdown`

**Status**: todo
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Summary

Delete `crates/runie-tui/src/markdown.rs` and render markdown via `tui-markdown` or `ratatui-markdown`.

## Acceptance Criteria

- `tui-markdown` is added to `runie-tui` dependencies.
- Custom markdown renderer is removed.
- Chat and markdown panels render through the crate while preserving theme integration.
- Code blocks remain syntax-highlighted.
- `cargo check --workspace` is green with no new warnings.

## Tests

- **Layer 3**: `TestBackend` + `Buffer` assertions for rendered markdown output (headings, lists, code blocks).
- **Layer 4**: Provider-replay test that streams a markdown response and verifies final rendered lines.
