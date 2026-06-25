# Externalize provider/model metadata as YAML

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Summary

Move provider and model metadata from static Rust arrays into YAML data files loaded at runtime (or embedded via `include_str!` with build-time validation). Users can add models without recompiling.

## Acceptance Criteria

- YAML model/provider metadata files live under `crates/runie-core/resources/`.
- `provider/registry_data.rs` and `model_catalog/*` static arrays are removed.
- A build script validates embedded metadata against a JSON Schema.
- User-provided metadata overrides are supported through the config directory.
- `cargo check --workspace` is green with no new warnings.

## Tests

- **Layer 1**: Parse and validate metadata files.
- **Layer 2**: Config loading with a custom user-provided model.
