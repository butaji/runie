# Session JSONL format

**Status**: todo

**Milestone**: MVP

**Category**: Sessions

## Description

Save/load sessions to JSONL files.

## Acceptance Criteria

- [ ] JSONL serialization per event
- [ ] File naming convention
- [ ] Metadata header
- [ ] Streaming read/write for large sessions

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
