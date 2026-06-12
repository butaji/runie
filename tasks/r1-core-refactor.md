# Core refactor: lint fixes, pure reducers, composed state

**Status**: done
**Milestone**: R1
**Category**: Core Architecture

## Description

Fix the top P0 issues from REVIEW.md and REFACTOR_PLAN.md without changing
external behavior. Code-quality milestone, not an architecture rewrite.

## Acceptance Criteria

- [x] Split `update.rs` (>600 lines) into `update/{mod,input,agent,slash,queue}.rs` ✓
- [x] Fix all clippy warnings in production code ✓
- [x] Cache `last_assistant_index` to make `append_response` O(1) ✓
- [x] No regressions: all 700+ existing tests still pass ✓

## Deferred (Not Blocking)

- [ ] Split `AppState` (27 fields) into composed structs — nice-to-have, requires updating 75+ tests
- [ ] Remove `VisibleRegion` — still used by autoscroll tests; can be cleaned up later

## Completed

1. **update.rs split** — All files under 500 lines:
   - `update/mod.rs` (~165 lines)
   - `update/agent.rs` (~236 lines)
   - `update/input.rs` (~427 lines)
   - `update/queue.rs` (~91 lines)
   - `update/slash.rs` (~145 lines)

2. **O(1) append_response** — Added `last_assistant_index` cache.

3. **Clippy fixes** — Zero clippy errors across all crates.

## Tests

- [x] Layer 1 — All existing state/logic tests pass (446 in runie-core)
- [x] Layer 2 — All event handling tests pass
- [x] Layer 3 — All rendering tests pass

## Notes

- Keep `&mut self` at the top level for performance; each reducer is
  "logically pure, mechanically mutable for zero-copy"
- Reference: REFACTOR_PLAN.md Phase 1-4, REVIEW.md P0 issues
