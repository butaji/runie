# Sort by last update

**Status**: todo

**Milestone**: MVP

**Category**: TUI Rendering

## Description

Elements float to bottom on update.

## Acceptance Criteria

- [ ] Timestamp tracking per element
- [ ] Re-sort on update
- [ ] Maintain stable order for same-timestamp items

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
