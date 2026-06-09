# Unified event type in runie-core


**⚠️ NOTE:** This task built code that is unused by the runtime. See `docs/SHIP_REVIEW.md`.
**Status**: done

**Milestone**: MVP

**Category**: Core Architecture

## Description

Unify all events into a single Event enum in runie-core. Remove the separate AgentEvent type with conversion logic.

## Acceptance Criteria

- [x] Single Event enum in runie-core::event
- [x] No AgentEvent type in runie-agent
- [x] All terminal input events included
- [x] All agent lifecycle events included
- [x] All tool events included

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
