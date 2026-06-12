# Session list/delete

**Status**: done

**Milestone**: MVP

**Category**: Sessions

## Description

List and delete sessions.

## Acceptance Criteria

- [x] List sessions by date (Store::list)
- [x] Delete confirmation (Store::delete)
- [x] Cascade deletion of related files (single file deletion)

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
