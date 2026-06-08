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
- [ ] Split `AppState` (28 fields) into composed structs: `InputState`, `ChatHistory`, `AgentState`, `UiState` (requires extensive test updates)
- [ ] Remove dead code: `VisibleRegion`, `visible_scroll()`, `visible()` (these are used in Snapshot, not truly dead)
- [x] No regressions: all 477+ existing tests still pass ✓

## Completed

1. **update.rs split** — Already done, files are under 500 lines:
   - `update/mod.rs` (165 lines)
   - `update/agent.rs` (236 lines)
   - `update/input.rs` (427 lines)
   - `update/queue.rs` (91 lines)
   - `update/slash.rs` (145 lines)

2. **O(1) append_response** — Added `last_assistant_index` cache:
   - `model.rs`: Added `last_assistant_index: Option<usize>` field
   - `update/agent.rs`: Updated `append_response` to use cached index
   - Updated index maintenance in `reorder_agent_after_tools` and `move_turn_complete_to_end`

3. **Clippy fixes** — Applied `cargo clippy --fix`:
   - Fixed unused imports, manual divisions, iterator methods
   - Applied 10+ auto-fixes in runie-core
   - Applied 1 fix in runie-agent

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes

## Notes

- Keep `&mut self` at the top level for performance; each reducer should be
  "logically pure, mechanically mutable for zero-copy"
- Do NOT rewrite the event bus or actor hierarchy — this task is about
  splitting files and structs, not changing runtime architecture
- Reference: REFACTOR_PLAN.md Phase 1-4, REVIEW.md P0 issues
