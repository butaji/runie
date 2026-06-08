# Input history

**Status**: in-progress

**Milestone**: MVP

**Category**: Input & Commands

## Description

Command history navigation.

## Acceptance Criteria

- [x] Up/Down arrows for history (history_prev/history_next)
- [ ] Persistent history across sessions (not implemented)
- [ ] Search/filter history (not implemented)

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
