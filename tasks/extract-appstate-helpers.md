# Extract `active_provider()` and `warn()` AppState helpers

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P3

**Depends on**: none
**Blocks**: none

## Description

Two boilerplate patterns repeat in `update/login_flow.rs` and other handlers: (1) `state.login_flow.as_ref().map(|f| f.provider.clone()).unwrap_or_default()` appears 2-3x to resolve the "current provider" — extract as `AppState::active_provider()`. (2) `state.set_transient(msg.into(), TransientLevel::Warning)` appears 4x in login flow and 3x in system handlers with the same shape — extract as `AppState::warn(msg: impl Into<String>)`. These are small but eliminate copy-paste and make intent explicit.

## Acceptance Criteria

- [ ] `AppState::active_provider(&self) -> String` method added, returns login_flow provider or config current_provider.
- [ ] `AppState::warn(&mut self, msg: impl Into<String>)` method added, calls `set_transient(msg, TransientLevel::Warning)`.
- [ ] All `state.login_flow.as_ref().map(|f| f.provider.clone()).unwrap_or_default()` call sites replaced with `state.active_provider()`.
- [ ] All `state.set_transient(msg.into(), TransientLevel::Warning)` call sites replaced with `state.warn(msg)`.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `active_provider_returns_login_flow_provider` — when login_flow is Some, returns flow.provider.
- [ ] `active_provider_returns_default_when_no_flow` — when login_flow is None, returns empty string (or config default).
- [ ] `warn_sets_transient_warning` — `state.warn("test")` sets a transient message at Warning level.

### Layer 2 — Event Handling
- [ ] `login_flow_reject_empty_key_uses_warn` — empty key submit calls `state.warn(...)` and stays on key input panel.
- [ ] `system_error_uses_warn` — system handler warning path uses the new helper.

### Layer 3 — Rendering
- [ ] N/A — transient message rendering unchanged.

### Layer 4 — Smoke / Crash
- [ ] N/A — no new IO or async paths.

## Files touched

- `crates/runie-core/src/model/state/app_state.rs` — add `active_provider()`, `warn()` methods
- `crates/runie-core/src/update/login_flow.rs` (or `login_flow/handlers.rs`) — replace 2-3 provider-chain + 4 warn call sites
- `crates/runie-core/src/update/system.rs` — replace 3 warn call sites
- `crates/runie-core/src/update/dialog/form.rs` — replace 2 warn call sites

## Notes

Small task but unblocks readability of login flow handlers. Pairs well with `consolidate-login-flow-handlers` — do that move first, then this cleanup on the consolidated file. Also consider adding `AppState::info(msg)` and `AppState::error(msg)` as companions if other TransientLevel values have similar boilerplate (audit `set_transient` call sites for `Info` and `Error` levels).
