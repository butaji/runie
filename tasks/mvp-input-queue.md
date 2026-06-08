# Message queue

**Status**: todo

**Milestone**: MVP

**Category**: Input & Commands

## Description

Message queue with steering and follow-up modes.

## Acceptance Criteria

- [ ] Enter - steering (queues message)
- [ ] Alt+Enter - follow-up (delivered after turn ends)
- [ ] Esc - abort and restore
- [ ] QueueAgent manages delivery timing

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
