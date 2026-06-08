# ANSI color support

**Status**: todo

**Milestone**: MVP

**Category**: TUI Rendering

## Description

Parse and render ANSI escape codes.

## Acceptance Criteria

- [ ] Parse ANSI codes from output
- [ ] Color restraint (terminal-safe)
- [ ] Bold/bright variants

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
