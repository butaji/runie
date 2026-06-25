# Split files at the 500-line limit (round 2)

**Status**: done
**Milestone**: R4
**Category**: Architecture / Refactoring
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

Scanned the files reported as "at the 500-line limit". Only `dialog/panel.rs` (498 lines) required splitting. The other three files (`update/input/text.rs` at 349 lines, `model/state/app_state.rs` at 88 lines, `config.rs` at 435 lines) were already well under the limit.

Split `dialog/panel.rs` into a `panel_split/` subdirectory:
- `panel.rs` (6 lines) — re-exports from `panel_split`
- `panel_split/mod.rs` (57 lines) — `Panel` struct, `PanelView`, `FormSubmitFn` types
- `panel_split/builders.rs` (268 lines) — fluent builder methods (new, list, form, item, toggle, select, field, etc.)
- `panel_split/navigation.rs` (146 lines) — selection, filtering, raw index helpers
- `panel_split/form_methods.rs` (61 lines) — form field methods, button accelerator lookup
- `panel_split/helpers.rs` (13 lines) — `normalize_title` helper

All resulting files are under 300 lines.

## Acceptance criteria

- [x] `dialog/panel.rs` split as described above.
- [x] No file in the workspace exceeds 400 lines after the split.
- [x] `cargo test --workspace` succeeds.
- [x] `cargo check --workspace` succeeds with no new warnings.
- [x] `build.rs` linter passes (zero violations).

## Tests

### Layer 4 — Smoke / Crash
- [x] `cargo test --workspace` green (all ~2060 tests) confirms no logic regression from the split.

## Files touched

- `crates/runie-core/src/dialog/panel.rs` — re-export module
- `crates/runie-core/src/dialog/panel_split/` — new subdirectory with 5 modules
- `crates/runie-core/src/dialog/mod.rs` — added `mod panel_split;`

## Notes

The other three files (`update/input/text.rs`, `model/state/app_state.rs`, `config.rs`) are currently under 350 lines and do not require splitting at this time. Task marked done.
