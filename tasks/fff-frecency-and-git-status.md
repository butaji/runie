# FFF Frecency and Git Status

**Status**: done
**Milestone**: R3
**Category**: Tools
**Priority**: P1

**Depends on**: fff-indexer-actor
**Blocks**: (none)

## Description

Feed file access from tools and the TUI into FFF's `FrecencyTracker` so search results rank recently/frequently used files higher. Expose git-status filters (`git:modified`, `git:untracked`, `git:staged`) in search queries.

## Acceptance Criteria

- [x] `read_file`, and `@` picker selections update frecency scores.
- [x] Search results include `git_status` fields.
- [x] Unified `search` tool accepts `git_status` filter (via `status:modified` syntax).
- [x] Frecency database is stored under Runie's cache directory.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `search_item_has_git_status_field` — `git_status` field serializes correctly.
- [x] `search_tool_schema_documents_git_filter` — description mentions git: filter.

### Layer 2 — Event Handling
- [x] `record_file_access` wired into `ReadFileTool` on successful read.
- [x] `record_file_access` wired into `@` picker via `insert_at_ref`.

### Layer 3 — Rendering
- [x] FFF file picker entries include `git_status` field (from `FffFileEntry`).

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/src/actors/fff_indexer.rs` — added `record_file_access`
- `crates/runie-core/src/tool/read_file.rs` — records frecency on read
- `crates/runie-core/src/update/dialog/mod.rs` — records frecency on picker selection
- `crates/runie-core/src/tool/search.rs` — git_status in SearchItem

## Notes

- Use git status from `fff-search`'s own watcher rather than shelling out to `git`.
- See `docs/adr/0023-fff-search-integration.md`.
