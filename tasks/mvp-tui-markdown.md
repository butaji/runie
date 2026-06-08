# Markdown rendering

**Status**: done

**Milestone**: MVP

**Category**: TUI Rendering

## Description

Render markdown in agent messages.

## Acceptance Criteria

- [x] Inline formatting (bold, italic, code)
- [x] Code blocks with language detection
- [x] Lists and blockquotes

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
