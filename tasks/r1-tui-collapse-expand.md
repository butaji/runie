# Ctrl+Shift+E collapse/expand

**Status**: todo

**Milestone**: R1

**Category**: TUI Improvements

## Description

Collapse/expand feed elements.

## Acceptance Criteria

- [ ] Ctrl+Shift+E keybinding
- [ ] Toggle element collapsed state
- [ ] Visual indicator for collapsed
- [ ] Restore expanded on expand

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
