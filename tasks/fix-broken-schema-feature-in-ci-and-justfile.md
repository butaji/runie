# Fix broken `schema` feature usage in CI and justfile

**Status**: todo
**Milestone**: R5
**Category**: Build / CI
**Priority**: P0

**Depends on**: none
**Blocks**: replace-config-validator-with-jsonschema

## Description

The CI workflow and `justfile` invoke `cargo run -p runie-core --example write_config_schema --features schema`, but `runie-core` does not declare a `schema` feature. `schemars` is already a normal dependency and the schema example is ungated, so the flag should be removed.

## Acceptance Criteria

- [ ] Remove `--features schema` from `.github/workflows/ci.yml` schema job.
- [ ] Remove `--features schema` from the `justfile` schema recipe.
- [ ] Remove or update the comment in `crates/runie-core/examples/write_config_schema.rs` that references the flag.
- [ ] The schema job/recipe runs successfully without the flag.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `schema_example_runs_without_feature_flag` — invoking the example without `--features schema` produces `config.schema.json`.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `.github/workflows/ci.yml`
- `justfile`
- `crates/runie-core/examples/write_config_schema.rs`

## Notes

- This is a prerequisite for `replace-config-validator-with-jsonschema` because CI must be able to generate the schema reliably.
- No new dependencies are required.
