# FFF Glob Tool

**Status**: todo
**Milestone**: R3
**Category**: Tools
**Priority**: P2

**Depends on**: fff-indexer-actor
**Blocks**: (none)

## Description

Add a fast `glob` tool backed by FFF. FFF’s glob implementation is reported to be 10–100× faster than typical Rust/Node glob libraries.

## Acceptance Criteria

- [ ] New `glob` tool accepts a pattern (e.g., `**/*.rs`) and returns matching paths.
- [ ] Supports pagination (`limit`/`page`).
- [ ] Results are structured JSON.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `glob_returns_matching_files` — `**/*.rs` returns Rust files.
- [ ] `glob_supports_negation` — `!test/` excludes directories.

### Layer 2 — Event Handling
- [ ] `glob_tool_emits_search_request` — tool call translates to a `SearchRequest`.

### Layer 3 — Rendering
- [ ] `glob_results_render_as_list` — TUI shows matched paths.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/src/tool/glob.rs` (new)
- `crates/runie-core/src/tool/mod.rs`

## Notes

- Consider merging this into the unified `search` tool with `mode = "glob"` instead of a separate tool.
- See `docs/adr/0023-fff-search-integration.md`.
