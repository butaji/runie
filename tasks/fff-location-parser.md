# FFF Location Parser

**Status**: done
**Milestone**: R3
**Category**: Tools
**Priority**: P2

**Depends on**: fff-indexer-actor
**Blocks**: (none)

## Description

Use FFF’s `file:line:col` parser for agent/TUI references like `@path/to/file.rs:42`. Ensures consistent, tested parsing across the codebase.

## Acceptance Criteria

- [x] Parse `file:line:col`, `file:line`, and `file` forms.
- [x] Validate that the parsed path exists in the index (or fallback to filesystem).
- [x] Expose a helper usable by both agent tools and the TUI.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `parser_extracts_line_and_column` — `src/lib.rs:10:5` → path, 10, 5.
- [x] `parser_handles_missing_column` — `src/lib.rs:10` → path, 10, None.
- [x] `parser_handles_no_location` — `src/lib.rs` → path, None.
- [x] `parser_handles_line_range` — `src/lib.rs:10-20` → path, range.
- [x] `parser_handles_column_range` — `src/lib.rs:10:5-20` → path, range.
- [x] `location_line_extraction` / `location_col` helpers.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/src/location.rs` (new) or `crates/runie-core/src/tool/mod.rs`
- Call sites that currently parse `file:line:col` manually.

## Notes

- This is a small refactor; prioritize after the unified search tool is working.
- See `docs/adr/0023-fff-search-integration.md`.
