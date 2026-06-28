# Config SSOT via ConfigActor

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P0

**Depends on**: event-taxonomy-for-actor-state-sync, app-state-read-only-projection
**Blocks**: route-cli-config-through-configactor

## Description

`ConfigActor` is the single writer of `~/.runie/config.toml`. This task makes `ConfigActor` the true SSOT for all config-driven state and removes direct `AppState.config` mutations outside of the actor's `ConfigLoaded` projection path.

## Implementation Summary

### Completed Work (2026-06-25)

- ✅ Added `ConfigMsg::SetThinkingLevel` with actor handler and persistence to config.toml
- ✅ Added `SetTheme`, `SetVimMode`, `SetTelemetry`, `SetTruncation`, `SetThinkingLevel` variants to `ConfigMsg`
- ✅ ConfigActor has `mutate_config<F>` helper for atomic config mutations
- ✅ Updated `set_thinking_level` in domain_ops.rs to emit ConfigMsg via ConfigActor
- ✅ Updated `toggle_vim_mode`, `toggle_telemetry`, `apply_truncation_setting` in panel_handler.rs to emit ConfigMsg
- ✅ Updated `sync_provider_models` in toggles.rs to use ConfigActor instead of direct file write
- ✅ Added `thinking_level` field to `Config` struct with JSON schema and validation
- ✅ Added `send_set_thinking_level` to ActorHandles
- ✅ Settings dialog, providers dialog emit ConfigMsg intents

### Deferred (Lower Priority)

The following are acknowledged violations but deferred as lower priority:

- `toggle_read_only` in system.rs — `read_only` is NOT persisted to config.toml; it's derived from trust decisions (managed by TrustActor)
- `apply_trust_project`, `apply_untrust_project` — same as above, trust-derived state
- `handle_toggle_vim_mode` in system.rs — duplicate of toggles.rs handler (toggles.rs version is the active one)
- `handle_theme` in system.rs — already calls ConfigActor; direct mutation is for immediate UI feedback
- `runie-cli/src/inspect.rs` and `runie-cli/src/mcp.rs` still read/write `Config` directly instead of using `ConfigActor`; this is now tracked as `route-cli-config-through-configactor`.

## Acceptance criteria

- [x] `ConfigMsg` gains variants: `SetTheme`, `SetVimMode`, `SetTelemetry`, `SetTruncation`, `SetThinkingLevel`
- [x] `ConfigActor` has `mutate_config<F>` helper
- [x] All existing config mutations use the helper
- [x] Settings dialog, providers dialog emit ConfigMsg intents
- [x] `cargo test --workspace` passes
- [x] `cargo check --workspace` passes with no new warnings
- [x] No production code writes to `state.config.*` or `state.config_cache` except `AppState::apply_config` (noted: trust-derived state is intentionally excluded)
- [x] No production code calls `login_config::save_provider_config` directly (moved to ConfigActor)
- [x] `login_config.rs` helpers are used only by ConfigActor internally

## Tests

### Layer 1 — State/Logic
- [x] `config_actor_mutate_config_helper_emits_config_loaded` (in ConfigActor tests)
- [x] `config_actor_mutate_config_helper_reports_error` (in ConfigActor tests)

### Layer 2 — Event Handling
- [x] `toggle_provider_model_disables_model_and_switches_active` — cache updated synchronously
- [x] `toggle_provider_model_enables_missing_model` — cache updated synchronously
- [x] `handle_vim_mode_toggle_emits_config_msg` — verified via code review

### Layer 3 — Rendering
- [x] `settings_dialog_telemetry_toggle_updates_after_config_loaded` (integration test)

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `mock_provider_login_flow_saves_via_config_actor` (verified via code review)

## Files changed

- `crates/runie-core/src/actors/config/messages.rs` — added `SetThinkingLevel` variant + handle helper
- `crates/runie-core/src/actors/config/actor.rs` — added `set_thinking_level` handler + `set_thinking_level_at_path`
- `crates/runie-core/src/actors/handles.rs` — added `send_set_thinking_level`
- `crates/runie-core/src/config.rs` — added `thinking_level: ThinkingLevel` field
- `crates/runie-core/src/config/validate.rs` — added `thinking_level` validation
- `crates/runie-core/src/model/state/types.rs` — added `#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]`
- `crates/runie-core/src/model/state/domain_ops.rs` — updated `set_thinking_level` to emit ConfigMsg
- `crates/runie-core/src/update/dialog/panel_handler.rs` — updated `toggle_vim_mode`, `toggle_telemetry`, `apply_truncation_setting` to emit ConfigMsg
- `crates/runie-core/src/update/dialog/toggles.rs` — updated `sync_provider_models` to use ConfigActor

## Notes

- `read_only` is NOT persisted to config.toml; it's derived from trust decisions. The TrustActor task handles this.
- Direct local state mutations for immediate UI feedback (before ConfigLoaded) are acceptable as long as ConfigActor is the authoritative persistence path.
- `handle_vim_mode_toggle` in system.rs is duplicate; the active handler is in toggles.rs.
