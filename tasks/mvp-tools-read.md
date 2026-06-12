# read tool

**Status**: done

**Milestone**: MVP

**Category**: Tools

## Description

Read file contents with line limits.

## Acceptance Criteria

- [x] Read file contents
- [x] Line limits (start/end offsets)
- [x] Binary file detection (via read_file_ref for images)
- [x] Error handling for missing/inaccessible files

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
