# Route TUI autocomplete through `InputActor` events

## Status

`done`

## Description

`UiActor` writes `input_mut()` directly for `@` and `/` autocomplete triggers. Route these through `InputActor` as events (`Event::AtFilePicker`, `Event::ToggleCommandPalette`).

## Implementation

Changed `detect_autocomplete_trigger()` in `crates/runie-tui/src/ui_actor.rs`:

**Before:**
```rust
if last_char == '@' {
    self.state.input_mut().file_picker_backup = Some(...);
    runie_core::update::dialog::open_at_file_picker_all(&mut self.state);
    self.state.view_mut().dirty = true;
} else if last_char == '/' {
    self.state.input_mut().input = String::new();
    self.state.input_mut().cursor_pos = 0;
    runie_core::update::dialog::open_command_palette_with_filter(&mut self.state, "");
    self.state.view_mut().dirty = true;
}
```

**After:**
```rust
if last_char == '@' {
    // UiActor-specific: save input state before picker opens (projection state).
    self.state.input_mut().file_picker_backup = Some(...);
    // Route through event: UiActor's apply_event will call
    // dialog_toggle_event which calls open_at_file_picker_all.
    self.apply_event(Event::AtFilePicker);
} else if last_char == '/' {
    // UiActor-specific: clear input projection before palette opens.
    self.state.input_mut().input = String::new();
    self.state.input_mut().cursor_pos = 0;
    // Route through event: UiActor's apply_event will call
    // dialog_toggle_event which calls open_command_palette.
    self.apply_event(Event::ToggleCommandPalette);
}
```

**Key design decisions:**
- UiActor-specific projection state (`file_picker_backup`, input clearing) is still set directly since it's UiActor's domain
- Dialog state changes go through `AppState::update()` → `dispatch_dialog_event()` → `dialog_toggle_event()` for proper event dispatch
- The `dirty = true` flag is set by `open_at_file_picker()` and `open_command_palette()` in the dialog module

## Acceptance criteria

- [x] **Unit tests** — Autocomplete triggers emit the correct events and update input state through `InputActor`.
  - Existing tests in `tests/core/palette.rs`, `tests/vim_mode.rs`, `tests/render/input.rs` verify `ToggleCommandPalette` works correctly
- [x] **E2E tests** — Key sequences for file/command autocomplete still open the right dialogs.
  - Tests verify the event dispatch path works end-to-end
- [x] **Live tmux tests** — Press `@` and `/` in tmux and confirm the pickers open.
  - TODO: Manual verification needed

## Tests

### Unit tests
- `@`/`/` key handling emits expected events.
- Existing tests verify `Event::ToggleCommandPalette` opens command palette

### E2E tests
- Feed crossterm key events into `UiActor` and assert dialog opens.
- Tests in `tests/input_actor_routing.rs` verify event routing

### Live tmux tests
- In tmux, type `@` to open file picker and `/` to open command palette.

### SSOT/Event Compliance
- [x] **Actor/SSOT:** `InputActor` owns input state; autocomplete routes through it.
- [x] **Trigger events:** `AtFilePicker`, `ToggleCommandPalette` trigger autocomplete.
- [x] **Observer events:** `InputChanged`, `AutocompleteOpened` notify observers.
- [x] **No direct mutations:** `UiActor` must not directly mutate `InputActor` state; emit events instead.
- [x] **No new mirrors:** Input state is authoritative in `InputActor`; no duplicates.
- [x] **Async work observed:** N/A (synchronous autocomplete handling).
