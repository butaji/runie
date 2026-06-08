# ConfigAgent

**Status**: todo

**Milestone**: R1

**Category**: Actor Architecture

## Description

Loads keybindings.json, TOML config, watches for changes.

## Acceptance Criteria

- [ ] Load keybindings.json
- [ ] Load config.toml
- [ ] File watcher for changes
- [ ] ConfigChanged events

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
