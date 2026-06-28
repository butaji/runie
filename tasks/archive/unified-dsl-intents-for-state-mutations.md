# Unified DSL intents for all state mutations

**Status**: done
**Milestone**: R4
**Category**: Input / Commands
**Priority**: P0

**Depends on**: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, declarative-actor-dsl
**Blocks**: none

## Progress

**Completed changes:**
- ✅ `/theme` now emits `Event::SwitchTheme` instead of directly setting `state.config_mut().theme_name`
- ✅ `/theme` with no args now returns `OpenDialog(DialogType::ThemeSelector)` which opens the theme selector via the event system
- ✅ `/theme` validates theme name and shows error for invalid themes
- ✅ `/reload` now emits `Event::ReloadAll` instead of directly calling `*state.skills_mut() = crate::skills::load_all()`
- ✅ Added `DialogType::ThemeSelector` to the DialogType enum
- ✅ Added `open_theme_selector` function in `update/dialog/open.rs` that properly builds the panel and sets state via the event system
- ✅ Session handlers (`run_name`, `run_fork`, `run_compact`) now emit intent events instead of directly mutating state
- ✅ `run_prompt` now emits `Event::RunPromptCommand` instead of directly calling `state.update()`
- ✅ `handle_settings` now emits `Event::ToggleSettingsDialog` instead of calling `open_settings_dialog()`

## Summary

Slash commands, palette commands, dialog actions, and keybindings have been standardized to emit intent events instead of directly mutating state. This completes the migration to event-driven state management.

## Acceptance criteria

- [x] All slash commands in `commands/dsl/handlers/` emit intent events only; they do not call `AppState` mutating helpers.
- [x] Session command handlers (`run_name`, `run_fork`, `run_compact`) emit intent events.
- [x] `run_prompt` handler emits intent events.
- [x] `handle_settings` emits `ToggleSettingsDialog` intent.
- [x] Command event handlers in `update/command.rs` contain the actual mutation logic.
- [x] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [x] `theme_switch_updates_state` — valid theme is set and message shown.
- [x] `theme_invalid_shows_error` — invalid theme shows error and doesn't change theme.

### Layer 2 — Event Handling
- [x] `theme_command_emits_set_theme` — `/theme runie` produces `SwitchTheme` event.
- [x] `theme_persisted_in_session` — theme changes are persisted in session.

## Files touched

- `crates/runie-core/src/commands/dsl/handlers/system.rs` — `/theme`, `/prompt`, `/settings` handlers now emit events
- `crates/runie-core/src/commands/dsl/handlers/session/run.rs` — session handlers emit events
- `crates/runie-core/src/update/command.rs` — event handlers contain mutation logic
- `crates/runie-core/src/update/system.rs` — `switch_theme` validates theme name
- `crates/runie-core/src/tests/theme_slash.rs` — updated test for new behavior
