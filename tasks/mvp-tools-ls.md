# ls tool

**Status**: done

**Milestone**: MVP

**Category**: Tools

## Description

List directory contents.

## Acceptance Criteria

- [x] List files and directories
- [x] Show file type indicators
- [x] Sorting (alphabetical)
- [x] Handle empty directories

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
