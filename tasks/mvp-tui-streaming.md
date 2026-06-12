# Streaming response merge

**Status**: done

**Milestone**: MVP

**Category**: TUI Rendering

## Description

Merge streaming responses by request ID.

## Acceptance Criteria

- [x] Request ID tracking (current_request_id in AppState)
- [x] Chunk accumulation per request (streaming flag and chunk events)
- [x] Merge into single message element (via UI layer)

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
