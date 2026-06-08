# Message queue

**Status**: done

**Milestone**: MVP

**Category**: Input & Commands

## Description

Message queue with steering and follow-up modes.

## Acceptance Criteria

- [x] Enter - steering (queues message) - via message_queue
- [x] Alt+Enter - follow-up (delivered after turn ends)
- [x] Esc - abort and restore (abort_queue function)
- [x] QueueAgent manages delivery timing (deliver_queued)

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
