# Extract `active_provider()` and `warn()` AppState helpers

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P3

**Depends on**: none
**Blocks**: none

## Description

Two boilerplate patterns repeat in `login_flow/handlers.rs` and other handlers: (1) `state.login_flow.as_ref().map(|f| f.provider.clone()).unwrap_or_default()` appears 4x to resolve the "current provider" — extracted as `AppState::active_provider()`. (2) `state.set_transient(msg.into(), TransientLevel::Warning)` appears 5x with the same shape — extracted as `AppState::warn(msg: impl Into<String>)`. These eliminate copy-paste and make intent explicit.

## Acceptance Criteria

- [x] `AppState::active_provider(&self) -> String` method added, returns login_flow provider or config current_provider.
- [x] `AppState::warn(&mut self, msg: impl Into<String>)` method added, calls `set_transient(msg, TransientLevel::Warning)`.
- [x] All `state.login_flow.as_ref().map(|f| f.provider.clone()).unwrap_or_default()` call sites replaced with `state.active_provider()`.
- [x] All `state.set_transient(msg.into(), TransientLevel::Warning)` call sites replaced with `state.warn(msg)`.
- [x] `cargo test --workspace` succeeds.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `active_provider_returns_login_flow_provider` — when login_flow is Some, returns flow.provider.
- [x] `active_provider_returns_config_default_when_no_flow` — when login_flow is None, returns config.current_provider.
- [x] `warn_sets_transient_warning` — `state.warn("test")` sets a transient message at Warning level.

### Layer 2 — Event Handling
- [x] `login_flow_reject_empty_key_uses_warn` — empty key submit calls `state.warn(...)` and stays on key input panel.
- [x] `system_error_uses_warn` — system handler warning path uses the new helper.

### Layer 3 — Rendering
- [ ] N/A — transient message rendering unchanged.

### Layer 4 — Smoke / Crash
- [ ] N/A — no new IO or async paths.

## Files touched

- `crates/runie-core/src/model/state/domain_ops.rs` — add `active_provider()` method
- `crates/runie-core/src/update/system.rs` — add `warn()` method, add test
- `crates/runie-core/src/login_flow/handlers.rs` — replace 4 provider-chain + 4 warn call sites
- `crates/runie-core/src/update/dialog/form.rs` — replace 2 warn call sites

## Notes

The `active_provider()` helper returns `login_flow.provider` when login_flow is active, or `config.current_provider` as fallback. This differs slightly from the original pattern which returned empty string, but provides a more useful fallback in the login flow context.
