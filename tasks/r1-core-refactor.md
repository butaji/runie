# Core refactor: lint fixes, pure reducers, composed state

**Status**: todo
**Milestone**: R1
**Category**: Core Architecture

## Description

Fix the top P0 issues from REVIEW.md and REFACTOR_PLAN.md without changing
external behavior. This is a code-quality milestone, not an architecture rewrite.

## Acceptance Criteria

- [ ] Split `update.rs` (>600 lines) into `update/{mod,input,agent,slash,queue}.rs`
- [ ] Split `AppState` (28 fields) into composed structs: `InputState`, `ChatHistory`, `AgentState`, `UiState`
- [ ] Fix all clippy warnings in production code
- [ ] Cache `last_assistant_index` to make `append_response` O(1)
- [ ] Remove dead code: `VisibleRegion`, `visible_scroll()`, `visible()`
- [ ] No regressions: all 477+ existing tests still pass

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
