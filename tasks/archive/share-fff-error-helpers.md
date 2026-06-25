# Share FFF error and lock helpers

**Status**: done
**Milestone**: R4
**Category**: Tools
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-engine/src/tool/search/core.rs` and `find_definitions.rs` duplicate `build_not_indexed_output`, `build_picker_not_initialized_output`, `with_picker`, and lock-guard error handling. Both wait on `FffSearchState::picker.read()` and produce a `ToolOutput` with the same JSON shape.

## Acceptance Criteria

- [ ] FFF-state accessors and error builders live in one shared module.
- [ ] `search/core.rs` and `find_definitions.rs` use the shared helpers.
- [ ] Lock-poisoning behavior and error JSON shape are unchanged.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `fff_helper_not_indexed_output_shape` — shared helper produces the expected JSON.
- [ ] `fff_helper_picker_acquired` — shared helper returns picker or not-indexed error.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `search_before_index_reports_not_indexed` — tool call before indexing uses shared helper.
- [ ] `find_definitions_before_index_reports_not_indexed` — same helper path.

## Files touched

- `crates/runie-engine/src/tool/search/core.rs`
- `crates/runie-engine/src/tool/find_definitions.rs`
- New `crates/runie-engine/src/tool/search/fff_helpers.rs` or `crates/runie-core/src/actors/fff_indexer/fff_helpers.rs`

## Notes

Coordinate with `dedupe-git-status-formatter` because the FFF indexer may move.
