# Orchestrator spawning all actors

**Status**: done

**Milestone**: MVP

**Category**: Core Architecture

## Description

Implement the Orchestrator as the central spawn point for all actors. The Orchestrator holds typed senders to each actor and routes messages accordingly.

## Acceptance Criteria

- [x] Orchestrator spawns AgentLoop, QueueAgent, SessionManager, ConfigAgent
- [x] Orchestrator holds typed channels to each actor
- [x] ToolActors spawned via Orchestrator on ToolStart events

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
