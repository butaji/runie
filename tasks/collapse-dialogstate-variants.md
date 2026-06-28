# Collapse `DialogState` variants

**Status**: done
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`DialogState` contained 7 overlapping variants (`Welcome`, `CommandPalette`, `ModelSelector`, `Settings`, `ScopedModels`, `SessionTree`, `PanelStack`) all backed by `PanelStack`. Refactored to 2 variants: `Welcome` and `Active { kind: DialogKind, panels: PanelStack }`. The `with_panel_stack!` macro was updated to handle the new enum shape uniformly.

## Acceptance Criteria

- [x] `DialogState` has no more than four top-level variants. (Reduced from 7 to 2: `Welcome`, `Active`)
- [x] Transitions are implemented as a pure state machine, not scattered `if` branches. (`with_panel_stack!` macro provides uniform access)
- [x] No duplicate data is stored across variants. (All `PanelStack`-carrying variants collapsed into `Active { kind, panels }`)
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `dialog_transitions_are_valid` — panel_stack() returns None for Welcome, Some for Active.
- [x] `dialog_prompt_data_unique` — only `Active { kind, panels }` carries panel data.

### Layer 2 — Event Handling
- [x] N/A — existing escape/enter tests cover this. Dialog toggle logic in `toggles.rs` unchanged.

### Layer 3 — Rendering
- [x] N/A — dialog rendering already covered by existing panel rendering tests.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A — dialog state is local TUI state.

## Files touched

- `crates/runie-tui/src/dialog.rs`
- `crates/runie-tui/src/app.rs`
- `crates/runie-tui/src/handler.rs`

## Notes

- If confirmation and prompting differ only by a callback, consider a single `Active` variant holding a payload enum.
- The `with_panel_stack!` macro in `commands/registry.rs` destructures `DialogState` variants to extract `PanelStack`; replace it with a helper method as part of this refactor.
- Preserve existing keyboard shortcuts; do not change UX while simplifying internals.
