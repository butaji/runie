# Delete Config Reload Shim

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: config-ssot-via-configactor
**Blocks**: consolidate-config-modules-into-dir

## Description

Remove the `config_cache` shim from `AppState`. With `ConfigActor` as the SSOT, config-driven state is now accessed via `ConfigState` which is updated via `ConfigLoaded` facts.

## Implementation Summary

### Completed Work (2026-06-25)

- ✅ Removed `config_cache: Option<Config>` field from `AppState`
- ✅ Added `model_providers: HashMap<String, ModelProvider>` to `ConfigState`
- ✅ Updated `apply_config()` to populate `ConfigState.model_providers` instead of `config_cache`
- ✅ Updated `configured_providers()`, `resolve_default_model()`, and `provider_config()` to use `ConfigState`
- ✅ Updated `login_flow/handlers.rs` to use `ConfigState`
- ✅ Updated `settings/dialog.rs` to use `ConfigState`
- ✅ Updated `update/dialog/toggles.rs` to use `ConfigState`
- ✅ Updated `update/system/model.rs` to use `ConfigState`
- ✅ Removed `config_cache` accessor methods from `accessors.rs`
- ✅ Removed `config_cache` lint rules from `build.rs`
- ✅ Updated all tests to use `ConfigState`

## Acceptance Criteria

- [x] `config_cache` field removed from `AppState`
- [x] Config state accessed via `ConfigState.model_providers`
- [x] All production code uses `ConfigActor` for config mutations
- [x] `cargo test --workspace` passes
- [x] `cargo check --workspace` succeeds with no new warnings

## Tests

### Layer 1 — State/Logic
- [x] `model_provider_operations_work` — verified through existing provider/model toggle tests
- [x] All existing tests pass with `ConfigState` instead of `config_cache`

### Layer 2 — Event Handling
- [x] Provider toggle tests verify synchronous state updates

### Layer 3 — Rendering
- [x] N/A

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] E2E login flow tests verify immediate model switching

## Files touched

- `crates/runie-core/src/model/state/app_state.rs` — removed `config_cache` field
- `crates/runie-core/src/model/state/session.rs` — added `model_providers` to `ConfigState`
- `crates/runie-core/src/model/state/accessors.rs` — removed `config_cache` accessors
- `crates/runie-core/src/model/state/domain_ops.rs` — updated to use `ConfigState`
- `crates/runie-core/src/model/state/session_restore.rs` — updated comments and implementation
- `crates/runie-core/src/login_flow/handlers.rs` — updated to use `ConfigState`
- `crates/runie-core/src/settings/dialog.rs` — updated to use `ConfigState`
- `crates/runie-core/src/update/dialog/toggles.rs` — updated to use `ConfigState`
- `crates/runie-core/src/update/system/model.rs` — updated to use `ConfigState`
- `crates/runie-core/src/provider/config.rs` — updated comments
- `crates/runie-core/build.rs` — removed `config_cache` lint rules
- `crates/runie-core/src/commands/tests/model.rs` — updated tests
- `crates/runie-core/src/login_flow/handlers_tests.rs` — updated tests
- `crates/runie-core/src/login_flow/e2e_tests.rs` — updated tests
- `crates/runie-core/src/update/dialog/tests/toggle_provider_tests.rs` — updated tests
- `crates/runie-core/src/update/system/tests.rs` — updated tests
- `crates/runie-tui/src/tests/mod.rs` — updated tests

## Notes

- Simple deletion task after `config-ssot-via-configactor` is complete
- `ConfigState.model_providers` replaces the redundant `config_cache` by storing the full provider configuration in the domain state
