# Collapse to single-line summary

**Status**: todo

**Milestone**: MVP

**Category**: TUI Rendering

## Description

Thinking and tool output collapse to single-line summary.

## Acceptance Criteria

- [ ] Thinking collapse to summary
- [ ] Tool output collapse to summary
- [ ] Expand on user interaction

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
