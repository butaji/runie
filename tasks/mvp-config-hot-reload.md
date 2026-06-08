# Hot reload on config change

**Status**: todo

**Milestone**: MVP

**Category**: Configuration

## Description

Reload configuration when files change.

## Acceptance Criteria

- [ ] File watcher
- [ ] ConfigChanged events
- [ ] Apply changes without restart

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
