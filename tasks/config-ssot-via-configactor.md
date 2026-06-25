# Config SSOT via ConfigActor

**Status**: in_progress
**Milestone**: R4
**Category**: Configuration
**Priority**: P0

**Depends on**: event-taxonomy-for-actor-state-sync, app-state-read-only-projection
**Blocks**: none

## Description

`ConfigActor` is the single writer of `~/.runie/config.toml`. This task makes `ConfigActor` the true SSOT for all config-driven state and removes direct `AppState.config` mutations outside of the actor's `ConfigLoaded` projection path.

**Progress (2026-06-25)**:
- ✅ Added `ConfigMsg::SetThinkingLevel` with actor handler and persistence to config.toml
- ✅ Updated `set_thinking_level` in domain_ops.rs to emit ConfigMsg via ConfigActor
- ✅ Updated `toggle_vim_mode`, `toggle_telemetry`, `apply_truncation_setting` in panel_handler.rs to emit ConfigMsg
- ✅ Updated `sync_provider_models` in toggles.rs to use ConfigActor instead of direct file write
- ✅ Added `thinking_level` field to `Config` struct with JSON schema and validation
- ✅ Added `send_set_thinking_level` to ActorHandles

**Remaining violations** (lower priority, deferred to later):
- `update/system.rs::toggle_read_only`, `apply_trust_project`, `apply_untrust_project`, `apply_initial_trust` — read_only is not persisted to config.toml; it's derived from trust decisions (managed by TrustActor in a separate task)
- `handle_toggle_vim_mode` in system.rs — duplicate of toggles.rs handler (toggles.rs version is dead code)
- `commands/dsl/handlers/system.rs::handle_theme` — already calls ConfigActor, direct mutation is for immediate UI feedback

## Acceptance criteria

- [x] `ConfigMsg` gains variants: `SetTheme`, `SetVimMode`, `SetTelemetry`, `SetTruncation`, `SetThinkingLevel`
- [x] `ConfigActor` has `mutate_config<F>` helper
- [x] All existing config mutations use the helper
- [ ] No production code writes to `state.config.*` or `state.config_cache` except `AppState::apply_config`
- [x] No production code calls `login_config::save_provider_config` directly (moved to ConfigActor)
- [ ] `login_config.rs` is deleted after test helpers are moved
- [x] Settings dialog, providers dialog emit ConfigMsg intents
- [x] `cargo test --workspace` passes
- [x] `cargo check --workspace` passes with no new warnings

## Tests

### Layer 1 — State/Logic
- [ ] `config_actor_mutate_config_helper_emits_config_loaded`
- [ ] `config_actor_mutate_config_helper_reports_error`

### Layer 2 — Event Handling
- [x] `toggle_provider_model_disables_model_and_switches_active` — cache updated synchronously
- [x] `toggle_provider_model_enables_missing_model` — cache updated synchronously
- [ ] `theme_intent_persists_through_actor`
- [ ] `vim_mode_intent_persists_through_actor`
- [ ] `thinking_level_intent_persists_through_actor`

### Layer 3 — Rendering
- [ ] `settings_dialog_telemetry_toggle_updates_after_config_loaded`

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `mock_provider_login_flow_saves_via_config_actor`

## Files changed

- `crates/runie-core/src/actors/config/messages.rs` — added `SetThinkingLevel` variant + handle helper
- `crates/runie-core/src/actors/config/actor.rs` — added `set_thinking_level` handler + `set_thinking_level_at_path`
- `crates/runie-core/src/actors/handles.rs` — added `send_set_thinking_level`
- `crates/runie-core/src/config.rs` — added `thinking_level: ThinkingLevel` field
- `crates/runie-core/src/config/validate.rs` — added `thinking_level` validation
- `crates/runie-core/src/model/state/types.rs` — added `#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]`
- `crates/runie-core/src/model/state/domain_ops.rs` — updated `set_thinking_level` to emit ConfigMsg
- `crates/runie-core/src/update/dialog/panel_handler.rs` — updated `toggle_vim_mode`, `toggle_telemetry`, `apply_truncation_setting` to emit ConfigMsg
- `crates/runie-core/src/update/dialog/toggles.rs` — updated `sync_provider_models` to use ConfigActor; moved tests to separate file
- `crates/runie-core/src/update/dialog/tests/` — new test directory with `toggle_provider_tests.rs`

## Notes

- `read_only` is NOT persisted to config.toml; it's derived from trust decisions. The TrustActor task will handle this.
- Direct local state mutations for immediate UI feedback (before ConfigLoaded) are acceptable as long as ConfigActor is the authoritative persistence path.
- `handle_vim_mode_toggle` in toggles.rs is dead code; the active handler is in system.rs.
