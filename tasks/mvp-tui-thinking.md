# Thinking text display

**Status**: todo

**Milestone**: MVP

**Category**: TUI Rendering

## Description

Display thinking/thought content from model.

## Acceptance Criteria

- [ ] AgentThinking event handling
- [ ] Thought content rendering
- [ ] Elapsed time display

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
