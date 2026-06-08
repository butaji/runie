# Agent crate module split

**Status**: todo
**Milestone**: R1
**Category**: Core Architecture

## Description

Split `runie-agent/src/lib.rs` (>600 lines) into focused modules. Keep tools
as synchronous functions — do NOT promote them to actors.

## Acceptance Criteria

- [ ] Extract `turn.rs` — agent turn loop and LLM interaction
- [ ] Extract `tools.rs` — tool enum and dispatch
- [ ] Extract `truncate.rs` — output truncation policies
- [ ] Extract `safety.rs` — bash validation (pure function, not an actor)
- [ ] Extract `parser.rs` — tool call parsing
- [ ] No file >500 lines, no function >40 lines
- [ ] All existing tests still pass

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes

## Notes

- Reference: REFACTOR_PLAN.md Phase 1
- Safety validation stays a pure function; no "SafetyAgent" actor needed
