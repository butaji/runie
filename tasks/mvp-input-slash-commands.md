# Slash commands

**Status**: todo

**Milestone**: MVP

**Category**: Input & Commands

## Description

Implement slash commands: /model, /save, /load, /sessions, /delete, /reset, /help, /compact

## Acceptance Criteria

- [ ] Command parsing
- [ ] /model - switch model
- [ ] /save, /load, /sessions, /delete - session management
- [ ] /reset - clear conversation
- [ ] /compact - context compaction
- [ ] /help - show available commands

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
