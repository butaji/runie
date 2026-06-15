# FFF TUI File Picker

**Status**: todo
**Milestone**: R3
**Category**: TUI / Rendering
**Priority**: P0

**Depends on**: fff-indexer-actor
**Blocks**: (none)

## Description

Replace the current `@` file picker backend with `fff-search` fuzzy file search. The picker ranks results by frecency and supports typo-tolerant queries.

## Acceptance Criteria

- [ ] `@` picker queries the `FffIndexerActor` instead of traversing directories.
- [ ] Results are ranked by frecency and fuzzy match score.
- [ ] Query input is typo-tolerant.
- [ ] Recently/frequently opened files appear near the top.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `picker_ranks_recent_files_higher` — frecency boosts recently accessed files.

### Layer 2 — Event Handling
- [ ] `picker_sends_search_request_on_input` — each keystroke publishes a debounced request.

### Layer 3 — Rendering
- [ ] `picker_renders_fuzzy_results` — TUI shows ranked list with highlights.

### Layer 4 — Smoke / Crash
- [ ] `smoke_at_picker` — run binary, open picker, select a file.

## Files touched

- `crates/runie-tui/src/widgets/file_picker.rs` (or existing picker module)
- `crates/runie-core/src/event/variants.rs`

## Notes

- Debounce input to avoid flooding the indexer.
- Visual indicator for recently modified files is optional.
- See `docs/adr/0023-fff-search-integration.md`.
