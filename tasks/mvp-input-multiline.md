# Multi-line input

**Status**: todo

**Milestone**: MVP

**Category**: Input & Commands

## Description

Support multi-line input editing.

## Acceptance Criteria

- [ ] Shift+Enter for newlines
- [ ] Ctrl+J for newlines
- [ ] Cursor positioning across lines
- [ ] Backspace at line start

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
