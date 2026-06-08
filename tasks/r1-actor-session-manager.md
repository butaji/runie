# SessionManager

**Status**: todo

**Milestone**: R1

**Category**: Actor Architecture

## Description

Handles session save/load/list/delete operations.

## Acceptance Criteria

- [ ] SaveSession event handling
- [ ] LoadSession event handling
- [ ] ListSessions event handling
- [ ] DeleteSession event handling

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
