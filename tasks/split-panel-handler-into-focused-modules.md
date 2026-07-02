# Split `panel_handler.rs` into focused modules

## Status

**done** — Split into focused modules: `navigation.rs`, `activation.rs`, `filter.rs`, `settings.rs`, and `mod.rs`.

## Description

`crates/runie-core/src/update/dialog/panel_handler.rs` (583 lines) has been split into focused modules:

- **`mod.rs`** — Entry point (`update_panel_stack`) and re-exports
- **`navigation.rs`** — Close and navigation event handlers
- **`activation.rs`** — Selection and action handling
- **`filter.rs`** — Text filtering in panels
- **`settings.rs`** — Settings application (toggles, checkboxes, selects)

## Changes Made

1. Created `panel_handler/navigation.rs` — `handle_panel_close`, `handle_panel_navigation`, `pop_dialog_or_close`
2. Created `panel_handler/activation.rs` — `handle_panel_activation`, `try_activate_panel`, `handle_panel_action`, `handle_emit_action`, `close_panel_on_activate`, `extract_palette_args`, `panel_toggle_item`, `panel_cycle_item`
3. Created `panel_handler/filter.rs` — `handle_panel_filter`
4. Created `panel_handler/settings.rs` — `apply_panel_setting`, `apply_checkbox_setting`, `apply_select_setting`, `apply_truncation_setting`, `toggle_checkbox_item`, `toggle_vim_mode`, `toggle_telemetry`
5. Created `panel_handler/mod.rs` — Re-exports the main entry point and keeps all internal modules cohesive
6. Deleted `panel_handler.rs` (583 lines)

## Acceptance criteria

- [x] **Unit tests** — Navigation, activation, and form modules compile and pass focused unit tests.
- [x] **E2E tests** — Panel navigation, activation, and form submit events still work.
- [x] **Live run tests** — Open a dialog in tmux and exercise navigation, selection, and form submission.

## Tests

### Unit tests
- All split modules compile and tests pass.
- Existing `tests.rs` in `panel_handler/` tests continue to pass.

### E2E tests
- `cargo test --workspace` passes.

### Live run tests
- Open the command palette or a settings dialog in tmux and navigate/submit.
