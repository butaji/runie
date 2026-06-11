# Form UX Redesign + Dialog Improvements

**Status**: in-progress
**Milestone**: R4
**Category**: TUI Rendering

## Description

Several dialogs have UX issues that need fixing with tests:

1. **Form dialogs** look like flat lists — need proper form layout with
   labels-above-inputs, input boxes, progress indicators, prominent submit
2. **Hotkey hint** must be pinned to the BOTTOM of every dialog (not pushed
   off-screen by long content)
3. **Scrollbars** must appear in every dialog when content overflows
4. **Word wrap** must be enabled for any text that may exceed dialog width
5. **Theme picker** must apply theme on Enter but keep dialog open (preview mode)
6. **Dialog variants** — unify by removing custom `DialogState` variants
   (`CommandPalette`, `ModelSelector`, `Settings`, `ScopedModels`,
   `SessionTree`) and using `PanelStack` everywhere

## Acceptance Criteria

- [x] Form fields render with label above input box (not inline)
- [x] Form shows progress indicator (① ② ③)
- [x] Form has prominent centered Submit button
- [x] Form shows cursor in active field
- [x] Theme picker applies theme on Enter, dialog stays open
- [x] Scoped models dialog has pinned hotkey hint (DONE in earlier commit)
- [x] All dialogs have scrollbar when content overflows
- [x] All dialogs have word wrap for long content
- [ ] All `DialogState` custom variants replaced with `PanelStack`

## Tests

### Layer 1 (state/logic)
- [ ] `theme_preview_keeps_dialog_open` — applying theme doesn't close picker
- [ ] `theme_preview_updates_config_immediately`

### Layer 2 (event handling)
- [ ] `enter_on_theme_item_switches_theme_but_keeps_dialog`
- [ ] `escape_closes_theme_picker`

### Layer 3 (rendering)
- [ ] `form_field_label_above_input` — label is on its own line above value
- [ ] `form_shows_progress_indicator` — ① ② markers visible
- [ ] `form_has_prominent_submit_button` — "Submit" visually distinct
- [ ] `form_shows_cursor_in_active_field`
- [ ] `form_completed_fields_show_checkmark`
- [ ] `scoped_models_has_pinned_hotkeys` (DONE)
- [ ] `model_selector_has_scrollbar_when_overflow`
- [ ] `settings_dialog_has_scrollbar_when_overflow`
- [ ] `session_tree_has_scrollbar_when_overflow`
- [ ] `all_dialogs_word_wrap_long_content`
