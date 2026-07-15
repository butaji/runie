# Dialog numeric selection

## Objective

Allow pressing number keys (`1-9`) to select options directly in permission dialogs and the model picker.

## Agent landscape finding

gemini-cli and kimi-code use number keys for direct selection in pickers, which is faster than arrow+Enter.

## runie current state

Dialogs support arrow keys, Tab, and Enter but not numeric selection.

## Required runie changes

- Map digit keys `1-9` to select the Nth visible option in permission dialogs
  (implemented as `_N` accelerators on the form buttons).
- Pressing a digit outside the visible range is ignored; the dialog stays open.
- Do not enable numeric selection in the command palette, where digits may be part of a filter string.
- **Model picker excluded** (decision): model names contain digits
  (`MiniMax-M2`, `gpt-4`, `claude-sonnet-4-6`), so digit keys are filter
  input there, not selection accelerators. The earlier draft of this task
  required numeric selection in the model picker; that requirement is
  withdrawn.

## Test scenarios

1. **Permission dialog: press 2 selects second option**
   - Keys: trigger permission dialog, press `2`
   - Assert: second option is selected and dialog closes (or advances).

2. **Model picker: digits are filter input**
   - Keys: open `/model`, press `1`
   - Assert: "1" enters the filter; with no matching model the picker shows
     the empty-filter state and stays open until Escape. (Digits cannot be
     accelerators here — model names contain digits.)

3. **Invalid digit is ignored**
   - Keys: open a 3-option dialog, press `9`
   - Assert: selection unchanged; dialog still open.

## Edge / negative cases

- Number keys still work for typing in filtered pickers when a filter string is active (deferred to command palette only).
- Double-digit numbers are not supported; `1` then `2` selects option 1, not 12.

## Dependencies

- `tool_permissions`
- `model_switching`

## Acceptance checklist

- [ ] All scenarios pass with `AppTest::mock()`.
- [ ] Edge cases are covered.
- [ ] No `sleep()` in resulting Rust tests.
- [ ] Tests use `keys::` constants.
