# FFF Indexer Actor

**Status**: todo
**Milestone**: R3
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: adopt-or-remove-actor-framework
**Blocks**: fff-unified-search-tool, fff-tui-file-picker, fff-find-definitions-tool, fff-frecency-and-git-status, fff-glob-tool, fff-location-parser

## Description

Create a long-lived `FffIndexerActor` that owns the shared `fff-search` state (`SharedFilePicker`, `SharedFrecency`, `SharedQueryTracker`). The actor indexes the workspace on startup, watches for filesystem changes, and answers search queries from tools and the TUI via the event bus.

## Acceptance Criteria

- [ ] `fff-search` is added as a dependency to `runie-core`.
- [ ] `FffIndexerActor` is created with lifecycle managed by the actor system.
- [ ] Actor initializes the index on workspace startup and waits for the initial scan.
- [ ] Actor handles `SearchRequest` events and returns `SearchResult` events.
- [ ] Actor shuts down cleanly, releasing LMDB/FFI resources.
- [ ] Memory usage is bounded by config (`max_index_memory_mb`).
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `indexer_initializes_in_temp_dir` — actor starts and completes a scan.
- [ ] `indexer_answers_file_search` — query returns expected file results.

### Layer 2 — Event Handling
- [ ] `search_request_event_returns_results` — send `SearchRequest`, receive `SearchResult`.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_indexer_startup_shutdown` — binary starts and exits without resource leaks.

## Files touched

- `crates/runie-core/Cargo.toml`
- `crates/runie-core/src/actors/fff_indexer.rs` (new)
- `crates/runie-core/src/event/variants.rs`
- `crates/runie-core/src/lib.rs`

## Notes

- Keep LMDB paths under the Runie data directory (e.g., `~/.cache/runie/fff/`).
- The MCP server path is intentionally out of scope; integration is native Rust.
- See `docs/adr/0023-fff-search-integration.md`.
