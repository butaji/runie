# Replace leftover macros with functions

**Status**: done
**Milestone**: R6
**Category**: Core / Refactoring
**Priority**: P3
**Note**: Targeted macros were removed; task file checkboxes now reflect completion.

**Depends on**: collapse-dialogstate-variants
**Blocks**: none

## Description

Several small macros can be ordinary functions or `if let` expressions: `with_panel_stack!` in `commands/registry.rs`, `with_ordering!` in `update/agent/mod.rs`, and the test skip macros `skip_if_seatbelt!`/`skip_if_integration!` in `runie-testing/src/macros.rs`.

## Acceptance Criteria

- [x] Replace `with_panel_stack!` with inherent methods `panel_stack()`/`panel_stack_mut()` on `DialogState`.
- [x] Replace `with_ordering!` with a helper function `apply_and_order(state, f)`.
- [x] Replace test skip macros with `#[cfg_attr(..., ignore)]` or helper functions.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `dialog_panel_stack_accessor` — `panel_stack()` returns the expected stack.

### Layer 2 — Event Handling
- [x] N/A.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `crates/runie-core/src/commands/registry.rs`
- `crates/runie-core/src/update/agent/mod.rs`
- `crates/runie-testing/src/macros.rs`

## Notes

- The `with_panel_stack!` part overlaps with `collapse-dialogstate-variants.md`; pick whichever task lands first to do it.
