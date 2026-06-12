# find tool

**Status**: done

**Milestone**: MVP

**Category**: Tools

## Description

Glob-based file finding with .gitignore support.

## Acceptance Criteria

- [x] Pattern matching (glob)
- [x] Path constraints
- [x] Max results limit
- [x] .gitignore awareness

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
