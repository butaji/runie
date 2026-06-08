# Streaming: event per chunk

**Status**: todo

**Milestone**: R1

**Category**: TUI Improvements

## Description

Each LLM chunk emitted as individual event.

## Acceptance Criteria

- [ ] ResponseChunk event per chunk
- [ ] ChatAgent accumulates chunks
- [ ] No buffering in Orchestrator

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
