# TOML configuration

**Status**: done

**Milestone**: MVP

**Category**: Configuration

## Description

Load configuration from TOML files.

## Acceptance Criteria

- [x] Parse ~/.runie/config.toml (Config::load)
- [x] Provider settings (model_providers)
- [x] Model preferences (provider, model, models.default)
- [x] Default values (Config::default)

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
