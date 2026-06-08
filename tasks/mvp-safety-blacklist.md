# Bash blacklist

**Status**: todo

**Milestone**: MVP

**Category**: Safety

## Description

Block dangerous bash commands.

## Acceptance Criteria

- [ ] Block rm -rf /
- [ ] Block dd, mkfs, fork bombs
- [ ] Pattern matching for variations
- [ ] SafetyAgent integration

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
