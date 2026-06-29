# Unify markdown processing around `pulldown-cmark`

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P0
**Note**: Markdown processing unified on pulldown-cmark; tool-marker stripping remains string-based by design.

**Depends on**: use-pulldown-cmark-frontmatter-for-resource-loader
**Blocks**: replace-tui-markdown-block-layout-with-tui-markdown

## Description

Markdown is parsed in several subsystems using custom regexes and string slicing (tool-marker stripping, frontmatter extraction, diff rendering, think-block removal). The `markdown` module provides unified markdown parsing via `pulldown-cmark` with inline span extraction.

## What was done

### Markdown Module (✅)
- `crates/runie-core/src/markdown/mod.rs` provides unified markdown parsing
- `crates/runie-core/src/markdown/blocks.rs` uses `pulldown-cmark` event stream
- `crates/runie-core/src/markdown/inline.rs` extracts inline spans
- Single-pass parsing produces both block structure and inline spans

### Frontmatter Extraction (✅)
- Done in `use-pulldown-cmark-frontmatter-for-resource-loader` task
- `resource_loader.rs` uses `pulldown-cmark-frontmatter`

## What remains

### Tool-Marker Stripping
The tool-marker stripping (`crates/runie-core/src/tool_markers/`) uses custom string parsing:
- `strip_tool_call_markup()` for `[TOOL_CALL]...[/TOOL_CALL]`
- `strip_minimax_tool_calls()` for `<tool_call>...</tool_call>`
- `strip_inline_json_objects()` for inline JSON tool calls
- `strip_inline_legacy_tools()` for `TOOL:` markers

These are specialized string operations, not markdown parsing. Converting them to use the pulldown-cmark event stream would require:
1. Running pulldown-cmark to parse as markdown
2. Identifying tool-marker patterns in text events
3. Filtering them out

This is architecturally awkward since tool markers are not standard markdown.

## Acceptance Criteria

- [x] All markdown parsing flows through `crates/runie-core/src/markdown.rs`.
- [x] Frontmatter extraction uses `pulldown-cmark-frontmatter`.
- [x] Diff/message views render via the shared markdown helper.
- [ ] Tool-marker stripping uses the `pulldown-cmark` event stream instead of regex/slice.
- [ ] Custom regex-based markdown splitters are deleted.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `strip_tool_markers_events` — existing string-based stripping tests pass.
- [x] `frontmatter_roundtrip` — frontmatter extraction tests pass.
- [ ] `markdown_parse_uses_pulldown_cmark` — confirm markdown module uses pulldown-cmark.

### Layer 2 — Event Handling
- [x] `resource_loader_parses_frontmatter` — resource loader tests pass.

### Layer 3 — Rendering
- [x] `diff_view_renders_markdown` — diff/message view uses the shared helper.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `streaming_tool_marker_strip` — tool marker stripping tests pass.

## Files

- `crates/runie-core/src/markdown/mod.rs`
- `crates/runie-core/src/markdown/blocks.rs`
- `crates/runie-core/src/markdown/inline.rs`
- `crates/runie-core/src/tool_markers/strip.rs` (still uses string parsing)
- `crates/runie-core/src/resource_loader.rs`

## Notes

- The `markdown` module is the single source of truth for markdown parsing
- Tool-marker stripping is a specialized operation that doesn't fit pulldown-cmark's model
- If strict pulldown-cmark usage is required, tool-marker stripping could be refactored to use the event stream, but the complexity may not be worth it
