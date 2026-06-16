# Coalesce Update Modules

**Status**: done
**Milestone**: R3
**Category**: Core / State
**Priority**: P1

**Depends on**: flatten-event-system, complete-appstate-refactor
**Blocks**: (none)

## Description

`crates/runie-core/src/update/` originally contained 27 modules totaling ~4,873 lines.
Dialog handling spanned `dialog.rs`, `dialog_form.rs`, `dialog_panel.rs`, and `dialog_toggle.rs`.
Input handling spanned `input_dispatch.rs`, `input_nav.rs`, `input_scroll.rs`, `input_text.rs`,
`input_text_support.rs`, `input_history.rs`, and `line_nav.rs`.

This fragmentation scattered related logic and forced contributors to guess which module owned a feature.

## What Was Done

### Consolidation

**Flat files (now 8, down from 11):**

| File | Role | Lines |
|------|------|-------|
| `agent.rs` | Agent lifecycle, model config, @-ref handling, scoped models | 593 |
| `login_flow.rs` | Login/authentication flow | 345 |
| `mod.rs` | Central dispatcher | 536 |
| `path_complete.rs` | Path completion standalone | 75 |
| `session.rs` | Session tree, save/load/import/export | 240 |
| `settings_dialog.rs` | Settings builder | 203 |
| `system.rs` | Control events, vim nav, model helpers | 421 |
| `tools.rs` | Bash, edit, edit_approval | 157 |

**`input/` subdir (4 modules):**

| File | Role | Lines |
|------|------|-------|
| `mod.rs` | `input_event` dispatcher + history/esc handlers | 115 |
| `nav.rs` | Cursor, vim nav, line nav | 339 |
| `scroll.rs` | Scroll events, element jump | 152 |
| `support.rs` | Grapheme boundaries, hints | 97 |
| `text.rs` | Insert/delete/paste/submit/undo/redo | 457 |

**`dialog/` subdir (5 modules):**

| File | Role | Lines |
|------|------|-------|
| `mod.rs` | `update_dialog`, form/panel routing, `process_command_result` | 407 |
| `form.rs` | Form panel actions | 136 |
| `model_selector.rs` | Partition model items | 35 |
| `panel.rs` | Panel stack navigation | 297 |
| `tab_complete.rs` | Tab key handler, ghost completion | 152 |
| `toggle.rs` | Dialog toggle events | 144 |

**Deleted:**
- `palette.rs` — dead code (`update_palette` never called; CommandPalette uses `panel::update_panel_stack`)
- `tab_complete.rs` → moved into `dialog/`
- `model_selector.rs` → moved into `dialog/`

### Fixes Applied

- `input/mod.rs` — restored `input_event` dispatcher with correct `pub fn`
- `scroll.rs` — fixed `nav::PAGE_SIZE` → `super::nav::PAGE_SIZE`
- `nav.rs` — fixed `crate::update::input::scroll::*` → `crate::update::input::*` (use re-exports)
- `system.rs` — fixed `crate::update::scroll_event` → `crate::update::input::scroll_event`
- `dialog/toggle.rs` — added `use super::*` import for `open_command_palette`, `toggle_dialog`, etc.
- `dialog/mod.rs` — re-exported `form_panel_action`, `apply_form_action`, `FormAction`, `dialog_toggle_event`
- `agent.rs` — removed duplicate `use crate::model::AppState`; added `dialog_toggle_event` import
- `session.rs` — removed duplicate `use crate::model::AppState`
- `tests/form_dialog.rs` — fixed `dialog_form::` → `dialog::form_panel_action` and `FormAction::`

## Acceptance Criteria

- [x] Update modules are grouped by domain into a small number of files
  (8 flat + 2 subdirs with 11 submodules total)
- [x] Dead/stub functions removed
- [x] `update/mod.rs` is the dispatcher
- [x] `cargo test --workspace` succeeds (1907+ tests, 0 failures)

## Tests

### Layer 1 — State/Logic
- [x] `update_module_count_reduced` — `ls crates/runie-core/src/update/*.rs` returns 8 files

### Layer 2 — Event Handling
- [x] `all_input_events_dispatch` — passes via existing test suite
- [x] `all_dialog_events_dispatch` — passes via existing test suite

## Files touched

- `crates/runie-core/src/update/*.rs`
- `crates/runie-core/src/update/input/`
- `crates/runie-core/src/update/dialog/`
- `crates/runie-core/src/tests/form_dialog.rs`
