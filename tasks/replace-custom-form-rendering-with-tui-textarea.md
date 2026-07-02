# Replace custom form rendering with `tui-textarea`/`tui-input`

## Status

`todo`

## Description

`popups/panel/form.rs` is a custom 400-line form renderer. Replace editable fields with `tui-textarea`/`tui-input` and buttons with `ratatui::widgets::List`.

## Acceptance criteria

1. **Unit tests** — Form state updates and validation work with the new widgets.
2. **E2E tests** — Form submit events produce the same results.
3. **Live tmux tests** — Open a settings/login form in tmux and fill/submit it.

## Tests

### Unit tests
- Field editing, validation, and submit logic.

### E2E tests
- Form submit events in replay.

### Live tmux tests
- Open a form dialog and edit fields.

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** N/A (UI-only change; `UiActor` state projection unchanged).
- [ ] **Trigger events:** `Submit` event triggers form submission.
- [ ] **Observer events:** Form state changes emit UI events via `UiActor`.
- [ ] **No direct mutations:** Form widget changes must not mutate actor-owned state directly.
- [ ] **No new mirrors:** Form state must be a projection, not authoritative storage.
- [ ] **Async work observed:** N/A (synchronous rendering).
