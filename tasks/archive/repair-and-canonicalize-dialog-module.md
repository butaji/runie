# Repair and canonicalize the dialog module

**Status**: done
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P0
**Depends on**: none
**Blocks**: gate-or-move-single-consumer-core-modules, unify-duplicate-module-names-core-tui

## Description

`crates/runie-core/src/dialog/` contained the canonical dialog module declared in `lib.rs`, but `crates/runie-tui/src/dialog/` was a duplicate subtree with minor differences. The key difference was that the TUI's `item.rs` defined an `EventLabel` trait while the core had `Event::default_label()` as a method. This task unified the dialog module by:

1. Moving the `EventLabel` trait to `runie_core::dialog::item` (making it public)
2. Updating `ItemAction::default_label()` to use the trait
3. Deleting the duplicate `runie-tui/src/dialog/` subtree
4. Updating `runie-tui/src/lib.rs` to re-export from `runie_core::dialog`

## Acceptance Criteria

- [x] `crates/runie-core/src/lib.rs` declares `pub mod dialog;` and exports `EventLabel` trait.
- [x] All dialog-related code in TUI imports from `runie_core::dialog` (or the renamed equivalent).
- [x] The duplicate `crates/runie-tui/src/dialog/` subtree is deleted.
- [x] `crates/runie-tui` imports all dialog types from `runie_core::dialog` and compiles.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] Dialog model structs (e.g., `PanelItem`, `ItemAction`) transition through their states according to business rules without any Ratatui imports.

### Layer 2 — Event Handling
- [x] Dialog-related events are routed to the correct dialog state mutations (verified by existing tests passing).

### Layer 3 — Rendering
- [x] Dialog widgets render correctly (verified by existing dialog_theme_switch tests passing).

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `dialog_module_compiles_and_tui_uses_core_types` — Verified by `cargo check --workspace` passing.

## Files touched

- `crates/runie-core/src/dialog/item.rs` (added `EventLabel` trait, updated `ItemAction::default_label()`)
- `crates/runie-core/src/dialog/mod.rs` (added `EventLabel` to re-exports)
- `crates/runie-tui/src/lib.rs` (removed `pub mod dialog;`, updated re-exports to use `runie_core::dialog`)
- `crates/runie-tui/src/dialog/` (deleted)

## Notes

The `EventLabel` trait was the only substantive difference between the two dialog implementations. Moving it to the core module unified both code paths while maintaining the same API surface.
