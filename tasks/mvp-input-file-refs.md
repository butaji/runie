# @-file reference detection

**Status**: done

**Milestone**: MVP

**Category**: Input & Commands

## Description

Detect and resolve @-file references.

## Acceptance Criteria

- [x] @-prefix detection
- [x] File path completion (complete_at_ref)
- [x] File content insertion (insert_at_ref)
- [x] Popup suggestion UI (at_suggestions in Snapshot, TUI popup)

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
