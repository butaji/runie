# Remove login_config cfg(test) shim from production AppState

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: actor-owned-state-ssot, config-ssot-via-configactor
**Blocks**: none

## Description

`AppState` has 5 `#[cfg(test)]` branches that fall back to `login_config::` for config reads/writes when `config_tx` / `config_cache` are `None`:

- `app_state.rs:267` `configured_providers()` → `login_config::list_configured_providers()`
- `app_state.rs:282` `resolve_default_model()` → `login_config::with_read_lock(|c| c.resolve_default_model())`
- `app_state.rs:294` `provider_config()` → `login_config::get_provider_config()`
- `app_state.rs:313` `remove_provider()` → `login_config::remove_provider_config()`
- `app_state.rs:325` `set_provider_models()` → `login_config::save_provider_config()`

This is a "test-helper compatibility shim" that leaks test-only code paths into production AppState. It violates domain purity: the `ConfigStore` trait already exists in `actors/config/store.rs` precisely to make config access injectable. ~15 test files call `login_config::set_test_config_*` / `set_test_config_with_providers` to seed the shim.

Replace the cfg(test) fallbacks with injected `ConfigStore` mocks. Tests that need seeded config construct an `AppState` with a `MockConfigStore` (or a pre-seeded `config_cache`) instead of poking global `login_config` state. Deletes `login_config.rs` (141 LOC) entirely.

## Acceptance Criteria

- [ ] All 5 `#[cfg(test)]` branches in `app_state.rs` removed.
- [ ] `login_config.rs` deleted (and `login_config/tests.rs`).
- [ ] `pub use login_config::{...}` removed from `lib.rs:129`.
- [ ] `crate::login_config::` imports in `update/login_flow.rs:199` and `update/dialog/provider_model_toggle.rs` replaced with `ConfigActor` messages or `config_cache` reads.
- [ ] ~15 test files rewritten to seed `AppState.config_cache` directly (or inject a `MockConfigStore`) instead of calling `login_config::set_test_config_*`.
- [ ] `rg "login_config" crates/` returns zero hits outside `tasks/`.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `configured_providers_reads_config_cache` — `AppState` with seeded `config_cache` returns the seeded providers without any global state.
- [ ] `resolve_default_model_reads_config_cache` — same for default model resolution.
- [ ] `provider_config_reads_config_cache` — same for single-provider lookup.

### Layer 2 — Event Handling
- [ ] `remove_provider_sends_config_msg` — `remove_provider` emits a `ConfigMsg::RemoveProvider` (no cfg(test) disk write).
- [ ] `set_provider_models_sends_config_msg` — `set_provider_models` emits `ConfigMsg::SetProviderModels`.

### Layer 3 — Rendering
- N/A — config access paths, not rendering.

### Layer 4 — Smoke / Crash
- [ ] `smoke_login_flow_without_login_config` — the `tests/login_logout/*` suite passes with the shim removed.

## Files touched

- `crates/runie-core/src/model/state/app_state.rs` — remove 5 cfg(test) branches
- `crates/runie-core/src/login_config.rs` → delete
- `crates/runie-core/src/login_config/tests.rs` → delete
- `crates/runie-core/src/lib.rs` — remove `pub mod login_config;` + re-exports
- `crates/runie-core/src/update/login_flow.rs` — replace `login_config::save_provider_config`
- `crates/runie-core/src/update/dialog/provider_model_toggle.rs` — replace `login_config::*`
- `crates/runie-core/src/tests/login_logout/*` — seed `config_cache` instead of `set_test_config_*`
- `crates/runie-core/src/tests/scoped_models.rs`, `tests/slash/model.rs`, `tests/slash/misc.rs`, `tests/model_selector.rs`, `commands/tests/*` — same

## Notes

The `ConfigStore` trait in `actors/config/store.rs` already provides `MockConfigStore`. The cleanest pattern: add a `test_config_cache(providers)` helper in `tests/mod.rs` that builds an `AppState` with a pre-seeded `config_cache: Option<Config>`. This replaces the global `login_config` state with per-test local state. Related: `consolidate-dual-path-modules` moves `login_config.rs` → `login_config/mod.rs` mechanically; this task deletes it entirely, so run this INSTEAD of the dual-path conversion for `login_config`.
