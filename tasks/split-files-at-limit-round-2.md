# Split files at the 500-line limit (round 2)

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Refactoring
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

Three files sit exactly at the 500-line limit and one at 467 — any edit breaks the build per `crates/runie-core/build.rs` linter (no allow-lists, current violations: 0). `split-large-files` (round 1, done) did not address these. Split each along its natural responsibility boundary so there is headroom for future edits. Targets: `update/input/text.rs` (500), `model/state/app_state.rs` (500), `dialog/panel.rs` (500), `config.rs` (467).

## Acceptance Criteria

- [ ] `update/input/text.rs` split into `text/` with submodules (e.g. `insert.rs`, `delete.rs`, `cursor.rs`), each <300 lines.
- [ ] `model/state/app_state.rs` split: inherent impl blocks moved to `app_state/{accessors.rs, mutations.rs}` or similar, struct def stays in `app_state.rs`, each file <300 lines.
- [ ] `dialog/panel.rs` split: `Panel` struct + `PanelView`/`PanelItem` in `panel.rs`, builder methods in `panel/builders.rs`, each <350 lines.
- [ ] `config.rs` split: move more sections into `config/{layers,schema}.rs` (already exist), target <350 lines for `config.rs` (or `config/mod.rs`).
- [ ] No file in the workspace exceeds 400 lines after the split (safety margin).
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `build.rs` linter passes (zero violations).

## Tests

### Layer 1 — State/Logic
- [ ] `app_state_accessors_preserved` — all existing AppState accessor tests pass after split.
- [ ] `config_sections_load` — config TOML parsing still works after section extraction.

### Layer 2 — Event Handling
- [ ] `text_input_insert_delete_cursor` — existing input tests (cursor, paste, multiline, undo, word_nav) pass after `text.rs` split.
- [ ] `panel_builder_produces_correct_view` — panel builder methods produce same `PanelView` after split.

### Layer 3 — Rendering
- [ ] N/A — no rendering logic in these files (config/state/input handlers are logic-only).

### Layer 4 — Smoke / Crash
- [ ] `cargo test --workspace` green (all ~2060 tests) confirms no logic regression from the split.

## Files touched

- `crates/runie-core/src/update/input/text.rs` → split into `text/{mod.rs, insert.rs, delete.rs, cursor.rs}`
- `crates/runie-core/src/update/input/tests.rs` — update if it references `text::` internal items
- `crates/runie-core/src/model/state/app_state.rs` → split struct def + impl blocks into `app_state.rs` + `state/{accessors.rs, mutations.rs}`
- `crates/runie-core/src/model/state/mod.rs` — update submodule declarations
- `crates/runie-core/src/dialog/panel.rs` → split into `panel.rs` (struct) + `panel/builders.rs` (methods)
- `crates/runie-core/src/config.rs` → move remaining sections into `config/schema.rs` or `config/layers.rs`
- `crates/runie-core/src/config/mod.rs` (or `config.rs`) — update declarations

## Notes

`split-large-files` (round 1, done) handled the previous batch. These 4 files either regressed or were missed. Do the splits mechanically (cut + paste + `use super::*`) — no logic changes. Priority is headroom, not elegance. Each split should leave 30%+ margin below 500. If `fold-state-into-model-state` runs first, `model/state/app_state.rs` may have new siblings — coordinate the split accordingly. `dialog/panel.rs` split pairs with `rename-update-dialog-panel` (which renames the OTHER `panel.rs` in `update/dialog/`) — these are different files, no conflict.
