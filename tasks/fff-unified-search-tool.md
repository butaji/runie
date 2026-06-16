# FFF Unified Search Tool

**Status**: done
**Milestone**: R3
**Category**: Tools
**Priority**: P0

**Depends on**: fff-indexer-actor
**Blocks**: fff-query-syntax-and-examples

## Description

Replace the separate `grep`, `find`, and `list_dir` tools with a single `search` tool backed by `fff-search`. The tool supports file search, content search, and mixed mode via a unified schema.

## Acceptance Criteria

- [ ] New `search` tool is registered in the tool registry.
- [ ] Tool schema exposes `query`, `mode` (`files` | `content` | `mixed`), `glob`, `git_status`, `limit`, and `page`.
- [ ] Results are returned as structured JSON (path, line, content, git status, frecency score) instead of plain text.
- [ ] Existing `grep`, `find`, and `list_dir` tools are deprecated or removed.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `search_finds_files_by_name` — fuzzy file search returns matches.
- [ ] `search_finds_content_by_pattern` — content search returns line matches.
- [ ] `search_applies_glob_constraints` — `*.rs` limits results.

### Layer 2 — Event Handling
- [ ] `search_tool_emits_search_request` — tool call publishes a `SearchRequest` event.

### Layer 3 — Rendering
- [ ] `search_results_render_as_tool_card` — TUI shows structured results.

### Layer 4 — Smoke / Crash
- [ ] `smoke_search_tool` — run binary, invoke search, verify results.

## Files touched

- `crates/runie-core/src/tool/search.rs` (new)
- `crates/runie-core/src/tool/mod.rs`
- `crates/runie-core/src/tool/grep.rs` (remove or deprecate)
- `crates/runie-core/src/tool/find.rs` (remove or deprecate)
- `crates/runie-core/src/tool/list_dir.rs` (remove or deprecate)

## Notes

- Constraint syntax (`*.rs !test/`) should be documented in tool examples.
- The legacy tools may be kept as thin aliases initially for compatibility.
- `ignore` + `globset` are intentionally not used; FFF covers `.gitignore`-aware traversal and glob matching.
- See `docs/adr/0023-fff-search-integration.md` and `docs/CRATE_DECISIONS.md`.
