# Config SSOT via ConfigActor

**Status**: in_progress
**Milestone**: R4
**Category**: Configuration
**Priority**: P0

**Depends on**: event-taxonomy-for-actor-state-sync, app-state-read-only-projection
**Blocks**: none

## Description

`ConfigActor` is already intended to be the single writer of `~/.runie/config.toml`, but most of the app bypasses it. This task makes `ConfigActor` the true SSOT for all config-driven state and removes every direct `AppState.config` mutation outside of the actor's `ConfigLoaded` projection path.

**Partial implementation**: Added ConfigMsg variants and handlers for theme, vim_mode, telemetry, and truncation. Updated `switch_theme` and `ToggleVimMode` handlers to emit intents. Still remaining: settings dialog, providers dialog, and login flow violations.

Current violators:
- `commands/dsl/handlers/system.rs::handle_theme` writes `state.config.theme_name`.
- `commands/dsl/handlers/model.rs::handle_thinking` / `run_thinking` write `state.config.thinking_level`.
- `update/system.rs::switch_theme`, `toggle_read_only`, `apply_trust_project`, `apply_untrust_project`, `apply_initial_trust` write `state.config.*`.
- `update/system.rs::control_event` toggles `state.config.vim_mode` directly.
- `update/system/model.rs::cycle_thinking_level` / `set_thinking_level` write `state.config.thinking_level`.
- `update/dialog/panel.rs::apply_panel_setting` writes `state.config.steering_mode`, `follow_up_mode`, `vim_mode`, `telemetry`, `truncation.*`.
- `update/dialog/toggle.rs::handle_vim_mode_toggle` writes `state.config.vim_mode`.
- `update/dialog/provider_model_toggle.rs` writes the config file directly via `login_config::save_provider_config`.
- `update/login_flow.rs::persist_login_flow` mutates `config_cache` directly via `sync_config_cache`.
- `update/dialog/toggle.rs::handle_providers_disconnect` mutates `config_cache` directly.

## Acceptance criteria

- [ ] `ConfigMsg` gains variants for every missing intent: `SetTheme`, `SetThinkingLevel`, `SetReadOnly`, `SetVimMode`, `SetTelemetry`, `SetTruncation { max_lines, max_bytes }`, `SetSteeringMode`, `SetFollowUpMode`.
- [ ] `ConfigActor` has a single `mutate_config<F>(&mut self, bus, f)` helper that does `spawn_blocking → save → load → emit ConfigLoaded`.
- [ ] All existing config mutations (`save_provider`, `remove_provider`, `set_default_model`, `set_provider_models`) use the helper.
- [ ] No production code writes to `state.config.*` or `state.config_cache` except `AppState::apply_config` (called from `update/mod.rs` on `Event::ConfigLoaded`).
- [ ] No production code calls `login_config::save_provider_config`, `login_config::remove_provider_config`, or `login_config::with_write_lock` directly.
- [ ] `login_config.rs` is deleted after test helpers are moved to `MockConfigStore` / seeded `config_cache` (see `remove-login-config-test-shim`).
- [ ] Settings dialog, providers dialog, and login flow all emit `ConfigMsg` intents; they do not write config themselves.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo check --workspace` passes with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `config_actor_mutate_config_helper_emits_config_loaded` — helper reloads config and emits the event.
- [ ] `config_actor_mutate_config_helper_reports_error` — I/O error produces `Event::Error` and does not emit `ConfigLoaded`.

### Layer 2 — Event Handling
- [ ] `theme_intent_persists_through_actor` — `SetTheme { name }` → `ConfigLoaded` updates `AppState.config.theme_name`.
- [ ] `vim_mode_intent_persists_through_actor` — `SetVimMode` toggles the saved flag.
- [ ] `provider_model_toggle_sends_set_provider_models` — toggling a model emits `ConfigMsg::SetProviderModels`, not a direct file write.
- [ ] `provider_disconnect_sends_remove_provider` — disconnect emits `ConfigMsg::RemoveProvider` and `ConfigLoaded` updates active model.

### Layer 3 — Rendering
- [ ] `settings_dialog_telemetry_toggle_updates_after_config_loaded` — toggling telemetry in settings changes the checkbox after the actor publishes the fact.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `mock_provider_login_flow_saves_via_config_actor` — add-provider flow persists through `ConfigMsg::SaveProvider` and the UI updates via `ConfigLoaded`.

## Files touched

- `crates/runie-core/src/actors/config/messages.rs` — new variants + handle helpers.
- `crates/runie-core/src/actors/config/actor.rs` — `mutate_config` helper; handlers for new variants.
- `crates/runie-core/src/actors/config/store.rs` — add blocking write helpers for theme/thinking/truncation/etc.
- `crates/runie-core/src/model/state/app_state.rs` — remove `config.*` writes outside `apply_config`.
- `crates/runie-core/src/update/system.rs` — `switch_theme`, `toggle_read_only`, trust helpers emit intents.
- `crates/runie-core/src/update/system/model.rs` — thinking-level helpers emit intent.
- `crates/runie-core/src/update/dialog/panel.rs` — settings toggles emit `ConfigMsg` intents.
- `crates/runie-core/src/update/dialog/toggle.rs` — vim-mode toggle emits intent.
- `crates/runie-core/src/update/dialog/provider_model_toggle.rs` — replace direct file write with `state.set_provider_models`.
- `crates/runie-core/src/update/login_flow.rs` — remove `sync_config_cache` optimistic update.
- `crates/runie-core/src/commands/dsl/handlers/system.rs` — `/theme` emits intent.
- `crates/runie-core/src/commands/dsl/handlers/model.rs` — `/thinking` emits intent.
- `crates/runie-core/src/login_config.rs` — delete after `remove-login-config-test-shim`.

## Notes

- This task supersedes and expands `dedupe-config-actor-mutations`, `consolidate-settings-providers-dialog` (config portion), and `remove-login-config-test-shim`. Update those tasks to reference this one.
- `state.config.model_source` is transient UI state, not persisted; it may still be set directly by `set_active_model` as long as the actor owns the persisted provider/model.
