# Scrollbar

**Status**: done

**Milestone**: MVP

**Category**: TUI Rendering

## Description

Scrollbar for message area.

## Acceptance Criteria

- [x] Thumb position based on scroll (scrollbar_metrics)
- [x] Track/thumb rendering (render_scrollbar function)
- [x] Handle dynamic content height (total_lines, offset)

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
