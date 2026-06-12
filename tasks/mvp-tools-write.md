# write tool

**Status**: done

**Milestone**: MVP

**Category**: Tools

## Description

Write complete file contents.

## Acceptance Criteria

- [x] Write full file contents
- [x] Create parent directories if needed
- [ ] Overwrite confirmation for existing files (not implemented - direct overwrite)
- [x] Error handling

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
