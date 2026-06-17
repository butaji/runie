# Extract Search Item Builder

**Status**: done
**Completed**: 2026-06-16
**Notes**: Extracted `build_search_item(path, git_status, score)` in `runie-engine/src/tool/search.rs` and reused it for fuzzy file search and glob search. Added 2 Layer 1 tests. cargo test --workspace passes.
**Milestone**: R4
**Category**: Tools
**Priority**: P2

**Depends on**: (none)
**Blocks**: (none)

## Description

Extract duplicated search item construction into a shared builder function.

**Duplicated code locations:**
- `crates/runie-core/src/tool/search.rs:271-293`
- `crates/runie-core/src/tool/search.rs:403-425`

Both blocks build `SearchItem` from search results with identical structure.

## Acceptance Criteria

- [ ] Shared `build_search_item` function extracted.
- [ ] Both locations updated to use shared function.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `build_search_item_creates_valid_item` — function produces valid SearchItem.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/src/tool/search.rs`
