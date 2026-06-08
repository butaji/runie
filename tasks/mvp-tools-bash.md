# bash tool

**Status**: done

**Milestone**: MVP

**Category**: Tools

## Description

Execute shell commands with safety guards.

## Acceptance Criteria

- [x] Execute bash commands via std::process::Command
- [x] Safety checks (SafetyAgent validation)
- [x] Output capture (stdout/stderr)
- [x] Timeout handling
- [x] Exit code handling

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
