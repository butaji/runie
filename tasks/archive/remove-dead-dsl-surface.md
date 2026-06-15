# Remove Dead DSL Surface and Dependency Cleanup

**Status**: done
**Completed**: 2026-06-14
**Milestone**: R3
**Category**: Core / State
**Priority**: P1

## Description

Several pieces of surface are unused or redundant after recent refactors:
- `CommandDef::form_panel` is never called.
- `ctrlc` workspace dependency is unused.
- `ratatui` `crossterm` feature is redundantly re-declared in `runie-tui` and `runie-term` Cargo.toml files.
- `DialogState` accessors duplicate 6-arm matches.

## Acceptance Criteria

- [ ] `CommandDef::form_panel` removed.
- [ ] `ctrlc` removed from workspace and crates.
- [ ] Crate Cargo.toml files use `ratatui.workspace = true`.
- [ ] `DialogState` refactored to avoid duplicated accessors.
- [ ] `REAL_PROVIDERS` alias cleaned up if still present.

## Tests

### Layer 1 — State/Logic
- [ ] `dialog_state_panel_stack_accessor`.
