# TUI render test cleanup

**Status**: todo
**Milestone**: R1
**Category**: TUI Rendering

## Description

Split oversized render test files and fix TUI-specific lint violations.

## Acceptance Criteria

- [ ] Split `runie-term/src/tests/render.rs` (>500 lines) into focused modules
- [ ] Split `runie-core/src/tests/` into feature-grouped modules if needed
- [ ] Fix any render functions >40 lines by extracting helpers
- [ ] All render tests still pass

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes

## Notes

- Reference: REFACTOR_PLAN.md Phase 1, Phase 4
- Goal is file-size compliance, not architectural rewrites
