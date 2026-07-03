# Consolidate `update/dialog/` thin handler files

**Status**: done
**Milestone**: R4
**Category**: Architecture / Refactoring
**Priority**: P2

**Depends on**: rename-update-dialog-panel
**Blocks**: none

## Description

`update/dialog/` has 11 handler files totaling 2,082 LOC — 29% of the entire `update/` directory. Several are thin (<150 LOC) toggle/picker helpers that fragment one coherent concern across multiple files. Group by responsibility: merge the toggle-family (`toggle.rs` 140 + `provider_model_toggle.rs` 118 + `model_selector.rs` 34 = 292) into one `toggles.rs`; merge the file-picker family (`fff.rs` 103 + `file_picker.rs` 91 = 194) into one `file_pickers.rs`. Keep `panel.rs`, `form.rs`, `form_handler.rs`, `form_tests.rs`, `open.rs`, `router.rs`, `tab_complete.rs` as-is (each is coherent or >150 LOC). Result: 11 files → 9.

## Acceptance Criteria

- [ ] `update/dialog/toggle.rs`, `update/dialog/provider_model_toggle.rs`, `update/dialog/model_selector.rs` deleted; contents merged into `update/dialog/toggles.rs` (≤400 LOC).
- [ ] `update/dialog/fff.rs`, `update/dialog/file_picker.rs` deleted; contents merged into `update/dialog/file_pickers.rs` (≤250 LOC).
- [ ] `update/dialog/mod.rs` updated: declares `toggles` and `file_pickers` instead of the 5 deleted modules.
- [ ] All imports of the deleted modules updated to the new module paths.
- [ ] `arch_guardrails.rs` path strings updated if it references any moved file.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `build.rs` linter passes (merged files under 500-line limit).

## Tests

### Layer 1 — State/Logic
- [ ] `toggle_state_flips_value` — existing toggle tests pass after merge into `toggles.rs`.
- [ ] `provider_model_toggle_updates_selected` — existing provider_model_toggle tests pass.
- [ ] `model_selector_cycles_models` — existing model_selector tests pass.
- [ ] `fff_picker_returns_results` — existing fff picker tests pass after merge.
- [ ] `file_picker_rebuilds_on_filter` — existing file_picker tests pass.

### Layer 2 — Event Handling
- [ ] `dialog_toggle_event_routes_to_toggles_module` — `DialogEvent::Toggle` routed correctly after merge.

### Layer 3 — Rendering
- [ ] N/A — handlers are logic-only; rendering covered by TUI tests.

### Layer 4 — Smoke / Crash
- [ ] `cargo test --workspace` green confirms all dialog event paths still route.

## Files touched

- `crates/runie-core/src/update/dialog/toggle.rs` → delete (merge into `toggles.rs`)
- `crates/runie-core/src/update/dialog/provider_model_toggle.rs` → delete (merge into `toggles.rs`)
- `crates/runie-core/src/update/dialog/model_selector.rs` → delete (merge into `toggles.rs`)
- `crates/runie-core/src/update/dialog/toggles.rs` → new
- `crates/runie-core/src/update/dialog/fff.rs` → delete (merge into `file_pickers.rs`)
- `crates/runie-core/src/update/dialog/file_picker.rs` → delete (merge into `file_pickers.rs`)
- `crates/runie-core/src/update/dialog/file_pickers.rs` → new
- `crates/runie-core/src/update/dialog/mod.rs` — update module declarations
- `crates/runie-core/tests/arch_guardrails.rs` — update path strings if referenced

## Notes

Depends on `rename-update-dialog-panel` (which renames `update/dialog/panel.rs` → `panel_handler.rs`) to avoid touching the same `mod.rs` twice. Rejected alternative: merge all 11 into one `handlers.rs` — rejected because `panel.rs` (422) + `form.rs` (357) + `router.rs` (118) are already coherent and the merged file would exceed 500 lines. `tab_complete.rs` (152) is input-completion, a different concern from toggles — leave it. Keep the merged files under 400 LOC to leave headroom.
