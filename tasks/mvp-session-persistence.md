# Session persistence across restarts

**Status**: todo

**Milestone**: MVP

**Category**: Sessions

## Description

Sessions persist across application restarts.

## Acceptance Criteria

- [ ] Auto-save on events
- [ ] Resume from last state
- [ ] Handle concurrent access

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
