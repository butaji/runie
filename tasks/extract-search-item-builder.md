# Extract Search Item Builder

**Status**: todo
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
