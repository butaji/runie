# Provider trait and trait objects

**Status**: done

**Milestone**: MVP

**Category**: Core Architecture

## Description

Define the Provider trait in runie-core, implemented by runie-provider. Support dynamic dispatch via trait objects.

## Acceptance Criteria

- [ ] Provider trait with generate method
- [ ] Message types (System, User, Assistant, ToolResult)
- [ ] ResponseChunk for streaming
- [ ] AnyProvider for dynamic dispatch

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
