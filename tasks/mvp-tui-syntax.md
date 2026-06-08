# Syntax highlighting

**Status**: in-progress

**Milestone**: MVP

**Category**: TUI Rendering

## Description

Syntax highlighting for code blocks.

## Acceptance Criteria

- [ ] Language detection
- [ ] Keyword highlighting
- [ ] Multiple language support

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
