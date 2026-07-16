# Approval dialog preview and feedback

## Objective

Add a full-screen diff/file preview (`Ctrl+E`) and inline rejection feedback to the permission dialog.

## Agent landscape finding

kimi-code supports `Ctrl+E` full-screen preview and a reject-to-feedback flow. codex uses `Y/N/A/D` approval keys.

## runie current state

Runie has a four-option permission dialog (Always / This session / Once / Deny). There is no preview or feedback input.

## Required runie changes

- Add `Ctrl+E` shortcut in the approval dialog to open a full-screen preview of the relevant diff or file content.
- Add a "Reject with feedback" option that switches to a small text area where the user can explain the rejection.
- Submitting feedback records a user message so the assistant can course-correct.

## Test scenarios

1. **Ctrl+E opens preview**
   - Keys: trigger edit permission dialog, press `C-e`
   - Assert: full-screen diff or file content is shown.

2. **Esc closes preview**
   - Keys: in preview, press `Esc`
   - Assert: returns to permission dialog.

3. **Reject with feedback**
   - Keys: select "Reject with feedback", type `do not change formatting`, press `Enter`
   - Assert: feedback appears as a user message in the transcript; no edit applied.

4. **Normal deny still works**
   - Keys: select "Deny" without feedback.
   - Assert: edit not applied; transcript shows a brief declined notice.

## Edge / negative cases

- Preview works for read, edit, and shell tool requests.
- Feedback is optional; rejecting without it does not open the input area.

## Dependencies

- `tool_permissions`

## Acceptance checklist

- [ ] All scenarios pass with `AppTest::mock()` or replay fixtures that trigger tool calls.
- [ ] Edge cases are covered.
- [ ] No `sleep()` in resulting Rust tests.
- [ ] Tests use `keys::` constants.
