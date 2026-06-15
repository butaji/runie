# FFF Frecency and Git Status

**Status**: todo
**Milestone**: R3
**Category**: Tools
**Priority**: P1

**Depends on**: fff-indexer-actor
**Blocks**: (none)

## Description

Feed file access from tools and the TUI into FFF’s `FrecencyTracker` so search results rank recently/frequently used files higher. Expose git-status filters (`git:modified`, `git:untracked`, `git:staged`) in search queries.

## Acceptance Criteria

- [ ] `read_file`, `edit_file`, and `@` picker selections update frecency scores.
- [ ] Search results include `frecency_score` and `git_status` fields.
- [ ] Unified `search` tool accepts `git_status` filter.
- [ ] Frecency database is stored under Runie’s cache directory.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `frecency_boosts_recently_read_file` — reading a file raises its search rank.
- [ ] `git_filter_returns_modified_files` — `git:modified` filter works in a dirty repo.

### Layer 2 — Event Handling
- [ ] `file_access_event_updates_frecency` — tool/UI publishes access event, frecency updates.

### Layer 3 — Rendering
- [ ] `git_status_badges_render_in_results` — modified/untracked badges shown.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/src/actors/fff_indexer.rs`
- `crates/runie-core/src/tool/read_file.rs`
- `crates/runie-core/src/tool/edit_file.rs`
- `crates/runie-tui/src/widgets/file_picker.rs`

## Notes

- Use git status from `fff-search`’s own watcher rather than shelling out to `git`.
- See `docs/adr/0023-fff-search-integration.md`.
