# Footer with token/queue count

**Status**: done

**Milestone**: MVP

**Category**: TUI Rendering

## Description

Show token count and queue count in footer.

## Acceptance Criteria

- [x] Token count display
- [x] Queue count display
- [x] Provider/model in status
- [x] Working indicator during streaming

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
