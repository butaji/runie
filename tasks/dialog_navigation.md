# Dialog Navigation

## Objective

Ensure all dialog types (form buttons, list panels, onboarding) support consistent keyboard navigation: Tab/Shift+Tab cycle focus, arrow keys move selection, and UP does not alter the chat input while a dialog is open.

## runie current state

All dialogs exist and are rendered by the TUI. The form/button and list/panel navigation is implemented.

## Test scenarios

### Form panels (permission dialog buttons)

1. **Tab navigates forward and wraps**
   - Open permission dialog, press Tab, press Enter
   - Assert: permission result shown.

2. **Arrow UP wraps to Always Allow**
   - Open permission dialog, press UP, press Enter
   - Assert: denied/deny shown.

3. **Allow accelerator keeps dialog open**
   - Open permission dialog, type "a" as filter char
   - Assert: dialog stays open.

### List panels (command palette)

4. **Arrow DOWN selects next command**
   - Open palette, press Down
   - Assert: selected command shows `▸` glyph.

5. **Tab selects next command**
   - Open palette, press Tab
   - Assert: selected command shows `▸` glyph.

6. **Shift+Tab selects previous command**
   - Open palette, press Down twice, Shift+Tab
   - Assert: selected command shows `▸` glyph.

7. **Arrow keys change selected command**
   - Open palette, press Down, then Up
   - Assert: first command selected again.

### List panels (model selector)

8. **Arrow DOWN selects next model**
   - Open model picker via /model, press Down
   - Assert: model selection moves.

9. **Tab and Enter confirm model selection**
   - Open model picker, press Tab, Enter
   - Assert: selection confirmed.

### List panels (settings dialog)

10. **Arrow DOWN navigates categories**
    - Open settings, press Down
    - Assert: selected row shows `▸` glyph.

11. **Tab navigates categories**
    - Open settings, press Tab
    - Assert: selected row shows `▸` glyph.

### List panels (providers dialog)

12. **Tab navigates provider actions**
    - Open providers dialog, press Tab
    - Assert: provider action selected.

### Onboarding (form panels)

13. **Onboarding provider picker arrow and Tab navigate**
    - Open onboarding, press Down, Shift+Tab
    - Assert: selected provider shows `▸` glyph.

### History and input recall

14. **UP recalls persisted history in empty input** (#[ignore])
    - Start with history `["persisted command"]`, press UP in empty input
    - Assert: prompt contains "persisted command".
    - **Note**: Requires `UiActor` to handle `Event::HistoryLoaded` and forward it to `InputActor` as `InputMsg::HistoryLoaded`. Covered by `tui_history_loaded_handling`.

15. **UP in permission dialog does not change chat input**
    - Start with history `["persisted command"]`, open permission dialog, press UP
    - Assert: dialog stays open, prompt unchanged.

## Edge / negative cases

- UP does not affect chat input while any dialog is open.
- Invalid keys (e.g., Ctrl combinations) are ignored in dialogs without crashing.
- Shift+Tab wrapping works correctly from first item.

## Dependencies

- `command_palette_navigation`
- `tool_permissions`
- `model_switching`
- `settings_dialog`
- `provider_management`
- `startup_onboarding`

## Acceptance checklist

- [x] All 15 scenarios implemented in `tests/dialog_navigation.rs`.
- [x] No `sleep()` in resulting Rust tests.
- [x] Tests use `keys::` constants.
- [x] Tests use `SHORT_TIMEOUT` / `MEDIUM_TIMEOUT` / `LONG_TIMEOUT`.
- [x] Session reuse intentionally disabled in this file (dialog state cannot be reliably reset between tests).
