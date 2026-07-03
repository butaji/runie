# Generate config.schema.json from code

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`config.schema.json` is checked in and mirrors defaults declared in `crates/runie-core/src/config.rs`. The schema is generated from the canonical Config type via `schemars::schema_for!(Config)`.

## Acceptance Criteria

- [x] Schema generation from `schemars::schema_for!(Config)` is the single source.
- [x] A CI check (`.github/workflows/ci.yml::schema`) fails if `config.schema.json` does not match generated output.
- [x] The checked-in file can be regenerated with one command: `cargo run -p runie-core --example write_config_schema --features schema`.
- [x] `cargo test --workspace` succeeds.
- [x] `generated_schema_matches_checked_in` test verifies the schema is in sync.

## Tests

### Layer 1 — State/Logic
- [x] `generated_schema_matches_checked_in` in `crates/runie-core/src/config/tests/schema_tests.rs`:
  - Reads the checked-in `config.schema.json`.
  - Generates schema from Config type via `schema::schema_json()`.
  - Asserts equality; fails with instructions to regenerate if out of sync.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / E2E
- N/A.

## Files touched

- `crates/runie-core/src/config/tests/schema_tests.rs` (new)
- `crates/runie-core/src/config/tests/mod.rs` (added schema_tests module)

## Implementation

1. Created `crates/runie-core/src/config/tests/schema_tests.rs` with `#[cfg(feature = "schema")]` gated test.
2. Added `mod schema_tests;` to `crates/runie-core/src/config/tests/mod.rs`.
3. Test uses `CARGO_MANIFEST_DIR` to resolve workspace root and `config.schema.json`.

## Notes

- CI already had a schema check job; this test ensures it never drifts silently.
- Regenerate command: `cargo run -p runie-core --example write_config_schema --features schema`
