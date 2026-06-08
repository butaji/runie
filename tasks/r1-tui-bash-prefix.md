# !command bash prefix

**Status**: todo

**Milestone**: R1

**Category**: TUI Improvements

## Description

Run bash and show output, don't send to agent.

## Acceptance Criteria

- [ ] ! prefix detection
- [ ] Run bash command
- [ ] Display output
- [ ] Don't add to message queue

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
