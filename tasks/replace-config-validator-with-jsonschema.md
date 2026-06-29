# Replace custom config validator with `jsonschema`

**Status**: partial
**Milestone**: R1
**Category**: Configuration
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/config/validate.rs` mixed a hand-written validator with `jsonschema`. The hand-written validator (field-by-field checks) was replaced with `jsonschema` validation against the `schemars`-generated schema. A separate `validate_registry` function for semantic checks (provider/model existence) remains in `validate.rs` since `jsonschema` cannot validate cross-references.

The remaining cleanup (delete `validate.rs`, inline `validate_registry` into callers) is tracked as a follow-up.

## Acceptance Criteria

- [x] Add `jsonschema` to `runie-core` dependencies (or workspace). — Done: `jsonschema` is in workspace dependencies
- [x] Implement JSON schema validation: serialize `Config` to `serde_json::Value`, then call `JSONSchema::compile(&schema).validate(&value)`. — Done: `validate.rs::validate_full()` uses `jsonschema`
- [x] Preserve the existing error-reporting shape (field path + message). — Done: errors include JSON pointer path and message
- [x] All existing config validation tests pass against the new validator. — Done: `config_actor_emits_error_on_invalid_config` and other tests pass
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

### Remaining (follow-up cleanup)

- [ ] Delete `crates/runie-core/src/config/validate.rs` and remove the module from `config/mod.rs`.
- [ ] Inline `validate_registry` (semantic provider/model existence check) into callers or a smaller helper.

## Tests

### Layer 1 — State/Logic
- [x] `valid_config_passes` — covered by existing config actor tests (`config_actor_loads_and_emits_config_loaded`)
- [x] `invalid_model_provider_rejected` — covered by `config_actor_emits_error_on_invalid_config`
- [x] `unknown_field_rejected` — `validate.rs::check_unknown_fields` flags unknown keys

### Layer 2 — Event Handling
- [x] `config_actor_emits_error_on_invalid_config` — `RactorConfigActor` emits `Event::Error` on validation failure

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `crates/runie-core/src/config/validate.rs` — hand-written validator replaced with `jsonschema::JSONSchema::compile(...).validate(...)`; `validate_registry` remains for semantic checks
- `crates/runie-core/src/config/mod.rs`
- `crates/runie-core/Cargo.toml` — `jsonschema` already in workspace
- `crates/runie-core/src/config/tests.rs`

## Notes

- `jsonschema` is a high-performance JSON Schema validator with broad draft support.
- The `config.schema.json` at the project root is generated from `schemars`; `validate.rs::schema_value()` returns the in-memory schema.
- `validate_registry` (semantic check: provider/model existence in registry) is separate from schema validation and cannot be expressed in JSON Schema; it remains in `validate.rs` as a follow-up cleanup.
- Status `partial`: the schema validation is done; file deletion + inline `validate_registry` is a follow-up task.
