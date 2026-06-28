# Unify markdown processing around `pulldown-cmark`

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P0

**Depends on**: use-pulldown-cmark-frontmatter-for-resource-loader
**Blocks**: replace-think-filter-with-regex

## Description

Markdown is parsed in several subsystems using custom regexes and string slicing (tool-marker stripping, frontmatter extraction, diff rendering, think-block removal). Consolidate all markdown handling on a single `pulldown-cmark` event stream and a small set of shared helpers.

## Acceptance Criteria

- [ ] All markdown parsing flows through `crates/runie-core/src/markdown.rs`.
- [ ] Tool-marker stripping uses the `pulldown-cmark` event stream instead of regex/slice.
- [ ] Frontmatter extraction uses `pulldown-cmark-frontmatter`.
- [ ] Diff/message views render via the shared markdown helper.
- [ ] Custom regex-based markdown splitters are deleted.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `strip_tool_markers_events` — event-stream stripping produces expected output.
- [ ] `frontmatter_roundtrip` — YAML frontmatter is extracted and body is preserved.

### Layer 2 — Event Handling
- [ ] `resource_loader_parses_frontmatter` — resource loader events yield parsed metadata.

### Layer 3 — Rendering
- [ ] `diff_view_renders_markdown` — diff/message view uses the shared helper and matches a `TestBackend` buffer.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `streaming_tool_marker_strip` — a captured provider stream with inline tool markers renders correctly end-to-end.

## Files touched

- `crates/runie-core/src/markdown.rs`
- `crates/runie-core/src/resource_loader.rs`
- `crates/runie-core/src/think.rs`
- `crates/runie-tui/src/ui/markdown.rs`
- `crates/runie-tui/src/ui/diff.rs`

## Notes

- Do not keep a fallback regex path; the event stream is the single source of truth.
- If `pulldown-cmark-frontmatter` is insufficient, fall back to `gray_matter` only after documenting why.
