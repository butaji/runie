# Core refactor: lint fixes, pure reducers, composed state

**Status**: in-progress
**Milestone**: R1
**Category**: Core Architecture

## Description

Fix the top P0 issues from REVIEW.md and REFACTOR_PLAN.md without changing
external behavior. This is a code-quality milestone, not an architecture rewrite.

## Acceptance Criteria

- [x] Split `update.rs` (>600 lines) into `update/{mod,input,agent,slash,queue}.rs` ✓
- [x] Fix all clippy warnings in production code ✓
- [x] Cache `last_assistant_index` to make `append_response` O(1) ✓
- [x] No regressions: all 700+ existing tests still pass ✓
- [ ] Split `AppState` (27 fields) into composed structs: `InputState`, `ChatHistory`, `AgentState`, `UiState`
- [ ] Remove dead code: `VisibleRegion`, `visible_scroll()`, `visible()` once autoscroll tests are updated

## Completed

1. **update.rs split** — Already done, all files under 500 lines:
   - `update/mod.rs` (~165 lines)
   - `update/agent.rs` (~236 lines)
   - `update/input.rs` (~427 lines)
   - `update/queue.rs` (~91 lines)
   - `update/slash.rs` (~145 lines)

2. **O(1) append_response** — Added `last_assistant_index` cache:
   - `model.rs`: Added `last_assistant_index: Option<usize>` field
   - `update/agent.rs`: Updated `append_response` to use cached index
   - Updated index maintenance in `reorder_agent_after_tools` and `move_turn_complete_to_end`

3. **Clippy fixes** — Applied `cargo clippy --fix`:
   - Fixed unused imports, manual divisions, iterator methods
   - Zero clippy errors across all crates

## Remaining Work

### AppState Composition
`AppState` currently has **27 public fields**. Split into:
```rust
pub struct AppState {
    pub input: InputState,      // text, cursor_pos, at_suggestions, etc.
    pub chat: ChatHistory,      // messages, scroll, etc.
    pub agent: AgentState,      // streaming, turn_active, inflight, queues, etc.
    pub ui: UiState,            // animation_frame, all_collapsed, etc.
}
```

**Blocked by:** 75+ tests in runie-core access AppState fields directly. This requires a large-scale test update.

### VisibleRegion Removal
`VisibleRegion` and `visible_scroll()` are still referenced by:
- `crates/runie-core/src/tests/autoscroll_bug.rs`
- `crates/runie-core/src/tests/autoscroll_overflow.rs`

These tests need to be rewritten to use `scroll_offset()` + `Paragraph::scroll()` before the struct can be removed.

## Tests

- [x] Layer 1 — All existing state/logic tests pass (446 in runie-core)
- [x] Layer 2 — All event handling tests pass
- [x] Layer 3 — All rendering tests pass
- [ ] Layer 4 — Smoke test after AppState split

## Notes

- Keep `&mut self` at the top level for performance; each reducer should be
  "logically pure, mechanically mutable for zero-copy"
- Do NOT rewrite the event bus or actor hierarchy
- Reference: REFACTOR_PLAN.md Phase 1-4, REVIEW.md P0 issues
