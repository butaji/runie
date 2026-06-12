# Sort by last update

**Status**: done

**Milestone**: MVP

**Category**: TUI Rendering

## Description

Elements float to bottom on update.

## Acceptance Criteria

- [x] Timestamp tracking per element (timestamp on ChatMessage)
- [x] Re-sort on update (messages_changed triggers rebuild)
- [x] Maintain stable order for same-timestamp items (tests verify this)

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
