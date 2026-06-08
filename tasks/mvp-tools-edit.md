# edit tool

**Status**: todo

**Milestone**: MVP

**Category**: Tools

## Description

Search/replace with unique match validation.

## Acceptance Criteria

- [ ] Find search pattern in file
- [ ] Unique match validation (error if multiple matches)
- [ ] Replace first occurrence only
- [ ] Error handling for missing patterns

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
