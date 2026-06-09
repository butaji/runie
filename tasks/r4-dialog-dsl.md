# Dialog DSL with Panel Stacking

**Status**: done
**Milestone**: R4
**Category**: TUI Rendering | Core Architecture

## Description

A declarative DSL for building dialog UIs with stackable panels for nested navigation.
Dialogs currently use ad-hoc per-variant state (`DialogState` enum) and rendering. This task
introduces a unified `Panel` type that can be composed into stacks, enabling nested navigation
within any dialog (e.g. Settings → Appearance → Theme → select → pop back).

Also fixes theme switching while a dialog is open — currently `SwitchTheme` events are swallowed
by `update_dialog()` and never reach `switch_theme()`.

## Acceptance Criteria

- [x] Theme can be switched while any dialog is open
- [x] `Panel` DSL supports: rows, toggles, selects, headers, separators
- [x] `PanelStack` manages push/pop navigation within a dialog
- [x] New `DialogState::PanelStack` variant renders via unified renderer
- [ ] Settings dialog migrated to panel DSL as proof of concept
- [x] Existing dialog behavior unchanged for non-migrated dialogs

## Tests

### Layer 1 — State/Logic
- [x] `panel_stack_push_pop` — push/pop changes active panel index
- [x] `panel_item_navigation_wraps` — up/down wraps around item list
- [x] `theme_switch_in_dialog_updates_config` — SwitchTheme reaches handler while dialog open

### Layer 2 — Event Handling
- [x] `settings_panel_navigates_items` — arrow keys move selection in panel
- [x] `settings_panel_select_toggles_value` — Enter toggles bool setting
- [x] `settings_panel_push_pop_panels` — nested panel navigation works

### Layer 3 — Rendering
- [x] `panel_dialog_renders_title_and_items` — TestBackend + Buffer assertion
- [x] `panel_stack_shows_breadcrumb` — nested panels show back button / path

### Layer 4 — Smoke
- [ ] `dialog_theme_switch_smoke.sh` — open settings, switch theme, verify no panic

## Notes

- Global events to pass through `update_dialog()`: `SwitchTheme`, `SwitchModel`, `Quit`
- Out of scope: migrating all dialogs (only Settings as PoC)
- Panel rendering reuses existing popup block + palette list styles
