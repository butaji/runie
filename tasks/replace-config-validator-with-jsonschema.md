# Replace custom config validator with `jsonschema`

**Status**: done
**Milestone**: R1
**Category**: Configuration
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

The `validate.rs` module mixed schema validation with semantic checks. The follow-up cleanup has been completed: `validate.rs` has been deleted and its functions (`validate`, `validate_registry`) are now inlined in `config_impl.rs`.

## Acceptance Criteria

- [x] Add `jsonschema` to `runie-core` dependencies (or workspace).
- [x] Implement JSON schema validation: serialize `Config` to `serde_json::Value`, then call `JSONSchema::compile(&schema).validate(&value)`.
- [x] Preserve the existing error-reporting shape (field path + message).
- [x] All existing config validation tests pass against the new validator.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.
- [x] Delete `crates/runie-core/src/config/validate.rs` and remove the module from `config/mod.rs`.
- [x] Inline `validate_registry` (semantic provider/model existence check) into callers.

## Tests

### Layer 1 — State/Logic
- [x] `valid_config_passes` — covered by existing config actor tests
- [x] `invalid_model_provider_rejected` — covered by config validation tests
- [x] `unknown_field_rejected` — `check_unknown_fields` flags unknown keys

### Layer 2 — Event Handling
- [x] `config_actor_emits_error_on_invalid_config` — `RactorConfigActor` emits `Event::Error` on validation failure

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `crates/runie-core/src/config/validate.rs` — **deleted** (functions inlined below)
- `crates/runie-core/src/config/config_impl.rs` — added `validate` and `validate_registry` functions inline
- `crates/runie-core/src/config/mod.rs` — removed `mod validate;`
- `crates/runie-core/src/config/tests/validate_tests.rs` — updated imports from `validate` module to `config_impl`
- `crates/runie-core/src/config/tests/mod.rs` — updated `crate::config::validate::validate` to `crate::config::config_impl::validate`

## Notes

- `validate` (JSON schema validation) and `validate_registry` (semantic provider/model checks) are now private helpers in `config_impl.rs`.
- Test imports updated to use `use crate::config::config_impl::{validate, validate_registry}`.
- 710 tests pass, workspace clean.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
