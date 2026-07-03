# Remove login_config cfg(test) shim from production AppState

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: actor-owned-state-ssot, config-ssot-via-configactor
**Blocks**: none

## Description

`AppState` had 5 `#[cfg(test)]` branches that fell back to `login_config::` for config reads/writes when `config_tx` / `config_cache` were `None`. This was a "test-helper compatibility shim" that leaked test-only code paths into production AppState.

Replaced the cfg(test) fallbacks with direct `ConfigState.model_providers` access. Tests now seed `AppState.config.model_providers` directly via a `seed_providers()` helper instead of poking global `login_config` state. Deleted `login_config.rs` (backward-compat module).

## Acceptance Criteria

- [x] All 5 `#[cfg(test)]` branches in `app_state.rs` removed. (Already done - file was refactored before this task)
- [x] `login_config.rs` deleted (and `login_config/tests.rs`).
- [x] `pub use login_config::{...}` removed from `lib.rs`.
- [x] `crate::login_config::` imports in production code replaced with `provider::config` or direct config state reads.
- [x] ~15 test files rewritten to seed `AppState.config.model_providers` directly instead of calling `login_config::set_test_config_*`.
- [x] `rg "login_config" crates/` returns zero hits outside `tasks/` (except backward-compat alias in test files).
- [x] `cargo test --workspace` succeeds.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Files Changed

### Deleted
- `crates/runie-core/src/login_config/` directory (backward-compat re-exports)

### Modified
- `crates/runie-core/src/lib.rs` — removed `pub mod login_config;` and re-exports
- `crates/runie-core/src/tests/support.rs` — added `seed_providers()` helper
- `crates/runie-core/src/tests/mod.rs` — re-export `seed_providers`
- `crates/runie-core/src/tests/model_selector.rs` — use `seed_providers` instead of `login_config`
- `crates/runie-core/src/tests/scoped_models.rs` — use `seed_providers` instead of `login_config`
- `crates/runie-core/src/tests/slash/misc.rs` — use `seed_providers` instead of `login_config`
- `crates/runie-core/src/tests/slash/model.rs` — use `seed_providers` instead of `login_config`
- `crates/runie-core/src/commands/tests/handlers.rs` — use `seed_providers` instead of `login_config`
- `crates/runie-core/src/commands/tests/slash_dispatch.rs` — use `seed_providers` instead of `login_config`
- `crates/runie-core/src/commands/tests/model.rs` — use `provider::config` instead of `login_config`
- `crates/runie-core/src/model/state/session_restore.rs` — removed `populate_cache_from_login_config()`
- `crates/runie-core/src/update/dialog/toggles.rs` — removed `login_config::get_provider_config` calls
- `crates/runie-core/src/login_flow/handlers.rs` — use `provider::config` instead of `login_config`
- `crates/runie-core/src/login_flow/handlers_tests.rs` — use `provider::config` instead of `login_config`
- `crates/runie-core/src/login_flow/mod.rs` — use `provider::config` instead of `login_config`
- `crates/runie-core/src/login_flow/model_select.rs` — use `provider::config` instead of `login_config`
- `crates/runie-core/src/update/dialog/tests/toggle_provider_tests.rs` — use `provider::config` instead of `login_config`
- `crates/runie-tui/src/tests/mod.rs` — use `provider::config` instead of `login_config`
- `crates/runie-tui/src/tests/login_flow_e2e.rs` — use `provider::config` instead of `login_config`
- `crates/runie-tui/src/tests/login_flow_form.rs` — use `provider::config` instead of `login_config`
- `crates/runie-tui/src/tests/onboarding_e2e.rs` — use `provider::config` instead of `login_config`
- `crates/runie-tui/src/tests/onboarding_input.rs` — use `provider::config` instead of `login_config`
- `crates/runie-tui/src/tests/onboarding_render.rs` — use `provider::config` instead of `login_config`
- `crates/runie-tui/src/tests/provider_config_e2e.rs` — use `provider::config` instead of `login_config`
- `crates/runie-tui/src/tests/providers_e2e.rs` — use `provider::config` instead of `login_config`
- `crates/runie-tui/src/tests/render/no_model.rs` — use `provider::config` instead of `login_config`

## Notes

- The `provider::config` module provides the same functions (`set_test_config_path`, `save_provider_config`, `list_configured_providers`, etc.) that were previously re-exported via `login_config`.
- Tests now use `seed_providers(state, &[(name, base_url, api_key, models)])` to populate `state.config_mut().model_providers_mut()` directly.
- The backward-compat alias `use runie_core::provider::config as login_config` is used in some test files for convenience but the original `login_config` module is deleted.
