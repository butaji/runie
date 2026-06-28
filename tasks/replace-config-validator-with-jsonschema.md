# Replace custom config validator with `jsonschema`

**Status**: todo
**Milestone**: R1
**Category**: Configuration
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/config/validate.rs` is a 325-line hand-written validator that must be kept in sync with the `schemars`-generated `config.schema.json`. The file’s own comment says it replaces `jsonschema`-based validation to remove the dependency. Re-adding `jsonschema` lets us validate the serialized `Config` value against the generated schema in a few lines, eliminating an entire file and its maintenance burden.

## Acceptance Criteria

- [ ] Delete `crates/runie-core/src/config/validate.rs` and remove the module from `config/mod.rs`.
- [ ] Add `jsonschema` to `runie-core` dependencies (or workspace).
- [ ] Implement validation as: serialize `Config` to `serde_json::Value`, then call `JSONSchema::compile(&schema).validate(&value)`.
- [ ] Preserve the existing error-reporting shape (field path + message) or document any UX change.
- [ ] All existing config validation tests pass against the new validator.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `valid_config_passes` — a known-good config validates without errors.
- [ ] `invalid_model_provider_rejected` — missing `api_key_env` / `base_url` produces the expected error path.
- [ ] `unknown_field_rejected` — extra keys in `model_providers` are flagged.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/config/validate.rs`
- `crates/runie-core/src/config/mod.rs`
- `crates/runie-core/Cargo.toml`
- `crates/runie-core/src/config/tests.rs` (if validation tests live there)

## Notes

- `ctx7` confirms `jsonschema` is a high-performance JSON Schema validator for Rust with broad draft support.
- The `config.schema.json` at the project root is generated from `schemars`; validation should use that exact file or the in-memory schema value.
- Rejected: keep the hand-written validator to avoid a dependency — correctness and synchronization with the schema outweigh the cost.
