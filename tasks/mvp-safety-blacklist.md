# Bash blacklist

**Status**: done

**Milestone**: MVP

**Category**: Safety

## Description

Block dangerous bash commands.

## Acceptance Criteria

- [x] Block rm -rf /
- [x] Block dd, mkfs, fork bombs
- [x] Pattern matching for variations
- [x] SafetyAgent integration (check_bash_safety called in run_bash)

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
