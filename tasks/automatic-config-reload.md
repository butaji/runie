# Automatic config reload

## Objective

Runie reloads configuration files automatically when they change, instead of requiring an explicit `/reload` command.

## Current state

- Runie already has a `/reload` command.
- File-system watching is not yet implemented.
- `tests/automatic_config_reload.rs` currently contains smoke tests that verify the app starts, responds to messages, and survives the `/reload` command; it does not yet write to config files during a test or assert that changes are detected automatically.

## Required runie changes

- Watch `~/.runie/config.toml` and `~/.runie/keybindings.toml` for changes.
- Reload keybindings, theme, and provider/model config when a change is detected.
- Do not interrupt an active turn; apply changes after the turn completes or at the next idle point.
- Debounce file-system events to avoid reloading on every save.

## Test scenarios

1. **Keybinding change takes effect without restart**
   - Setup: `AppTest::mock()` start, then write a new keybinding to `~/.runie/keybindings.toml` in the temp home.
   - Assert: the new keybinding works within `TimeoutConfig::response`.

2. **Theme change applies automatically**
   - Setup: change the theme in `~/.runie/config.toml` while Runie is running.
   - Assert: UI reflects the new theme in the captured tmux pane.

3. **Reload does not abort active turn**
   - Setup: submit a message to start a turn, then modify `~/.runie/config.toml`.
   - Assert: response completes; config applies afterward.

## Edge / negative cases

- Malformed config file is rejected with an inline error; previous config remains active.
- Deleted config file falls back to defaults without crashing.

## Dependencies

- `input_composition`
- `command_palette_navigation`

## Acceptance checklist

- [ ] All scenarios pass with `AppTest::mock()` and filesystem manipulation in the temp home.
- [ ] Edge cases are covered.
- [ ] No `sleep()` in resulting Rust tests.
