# Shared event bus with typed channels


**⚠️ NOTE:** This task built code that is unused by the runtime. See `docs/SHIP_REVIEW.md`.
**Status**: done

**Milestone**: MVP

**Category**: Core Architecture

## Description

Implement the shared event bus that all actors communicate through. The bus supports typed channels per actor, event tagging (domain vs ephemeral), and subscription filtering.

## Acceptance Criteria

- [x] EventBus struct with publish/subscribe methods
- [x] Typed channels per actor (each actor has its own channel)
- [x] Events tagged as domain (persisted) or ephemeral (not persisted)
- [x] Domain events: Submit, SpawnAgent, AgentThinking, AgentResponse, ToolStart, ToolEnd, Done, SwitchModel, ToolRegistered
- [x] Ephemeral events: ScrollUp, CursorLeft, CursorRight, Paste, ToggleExpand, etc.

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
