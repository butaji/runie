# QueueAgent

**Status**: todo

**Milestone**: R1

**Category**: Actor Architecture

## Description

Manages message queue with configurable batching.

## Acceptance Criteria

- [ ] Queue messages while agent busy
- [ ] one-at-a-time mode
- [ ] all mode (batch delivery)
- [ ] Emit SpawnAgent when ready

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
