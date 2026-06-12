# Output truncation

**Status**: done

**Milestone**: MVP

**Category**: Tools

## Description

Truncate tool output with lines and bytes limits.

## Acceptance Criteria

- [x] Line count limits
- [x] Bytes limits
- [x] Head/tail truncation modes
- [x] Truncation indicators in output

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
