# Task: Fix @-File Picker UX Issues

## Context
The @-file picker dialog was implemented in the previous commit but has several UX issues that need fixing.

## Issues

### Issue 1: Dialog title has redundant "@"
The dialog title shows ` @ files ` — the @ is redundant since the user already typed @ to open it. Should be ` Files `.

### Issue 2: Old `@-suggestions` popup interferes after dialog closes
The legacy `at_suggestions` completion system (from `at_refs.rs`) still activates when the input contains `@`. After the dialog closes and inserts `@Cargo.toml`, if the user keeps typing, the old popup appears over the input. The old system must be fully disabled.

### Issue 3: Dialog inserts `@filename` instead of just the filename
When the user types `@` → dialog opens → selects `Cargo.toml`, the input should be `@Cargo.toml` (the @ stays, the filename is appended). Currently this works, BUT if the user typed `@car` before the dialog opened (in future), it should replace the partial query. For now, ensure clean insertion: dialog should close cleanly and the input should have the filename appended after the existing `@`.

## Acceptance Criteria

### Layer 1: State/Logic
- [x] `open_at_file_picker` creates a dialog with title ` Files ` (no @)
- [x] `insert_at_ref` closes dialog and inserts `@path` into input
- [x] `handle_at_trigger` is disabled — no `at_suggestions` popup ever appears

### Layer 2: Event Handling
- [x] Typing `@` with empty input opens dialog (not old popup)
- [x] Typing `@` with non-empty input is treated as normal text (no popup)
- [x] After dialog selection, input contains `@filename` and no popup appears

### Layer 3: Rendering
- [x] No `@-suggestions` popup renders anymore
- [x] Dialog renders with title ` Files `

### Tests
- [x] `at_ref_dialog_title_has_no_at_symbol`
- [x] `at_ref_does_not_trigger_old_popup`
- [x] `at_ref_after_dialog_no_popup`

## Dependencies
None (fixes existing feature).
