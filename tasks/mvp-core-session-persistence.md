# Session persistence with event log

**Status**: todo

**Milestone**: MVP

**Category**: Core Architecture

## Description

Persist sessions as event logs. Save domain events to JSONL files, replay on load.

## Acceptance Criteria

- [ ] Domain events serialized to JSONL
- [ ] Session load replays events into all actors
- [ ] SessionManager handles save/load/list/delete
- [ ] Periodic snapshots as load accelerators

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
