# Configurable keybindings

**Status**: todo

**Milestone**: R1

**Category**: Configuration

## Description

Load keybindings from keybindings.json via ConfigAgent.

## Acceptance Criteria

- [ ] Parse keybindings.json
- [ ] Default keybindings
- [ ] Hot reload on change
- [ ] All actors subscribe to updates

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
