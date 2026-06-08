# ToolActors

**Status**: todo

**Milestone**: R1

**Category**: Actor Architecture

## Description

Spawn ToolActor per tool invocation. ToolActors self-describe via ToolRegistered event.

## Acceptance Criteria

- [ ] ToolActor per invocation
- [ ] ToolRegistered event on spawn
- [ ] ToolStart/ToolEnd events
- [ ] Orchestrator spawns ToolActors

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
