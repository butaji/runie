# Complete AppState Refactor

**Status**: done
**Milestone**: R3
**Category**: Core / State
**Priority**: P1

**Depends on**: (none)
**Blocks**: coalesce-update-modules

**Re-opened**: 2026-06-16 — `AppState` still mixes singleton UI flags with sub-states and contains display-only fields; the refactor is incomplete despite Phase comments being removed.

## Description

`AppState` was partially decomposed into sub-states (`InputState`,
`AgentState`, `ViewState`, `SessionState`, `ConfigState`, `CompletionState`)
in a “Phase 1” migration that never finished. The code still contains
duplicated fields (e.g. `input_history` appears twice), comments like
“Phase 1: add without removing outer fields,” and mixed access patterns such
as `state.config.current_model` alongside direct `AppState` fields like
`vim_nav_mode`.

This task finishes the migration so every piece of state has exactly one
home.

## Acceptance Criteria

- [x] Every field on `AppState` either belongs to one of the six sub-states
  or is explicitly documented as a cross-cutting singleton flag.
- [x] Duplicated fields are removed and all references updated.
- [x] The `Phase 1` comments are deleted.
- [x] `AppState` becomes a thin container over the sub-states plus a few
  documented singleton flags.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `appstate_fields_have_single_home` — grep confirms no field name
  appears in both `AppState` and a sub-state.
- [x] `default_appstate_matches_substate_defaults` —
  `AppState::default()` is equivalent to the composition of sub-state
  defaults.

### Layer 2 — Event Handling
- [x] `input_event_updates_only_input_state` — an input event mutates only
  `state.input`.
- [x] `model_config_event_updates_only_config_state` — a model config event
  mutates only `state.config`.

## Files touched

- `crates/runie-core/src/model/state.rs`
- `crates/runie-core/src/state.rs`
- `crates/runie-core/src/update/*.rs`
- `crates/runie-core/src/lib.rs`

## Notes

Moved `vim_nav_mode`/`vim_nav_pending` to `ViewState`,
`file_picker_backup`/`file_picker_range_suffix` to `InputState`, and
`command_usage` to `ConfigState`. Updated all call sites in `update/`,
`snapshot.rs`, `commands/tests/usage.rs`, and TUI/render tests.
`cargo test --workspace` passes.
