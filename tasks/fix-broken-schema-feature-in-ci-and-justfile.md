# Fix broken `schema` feature usage in CI and justfile

**Status**: done
**Milestone**: R5
**Category**: Build / CI
**Priority**: P0

**Depends on**: none
**Blocks**: replace-config-validator-with-jsonschema

## Description

The CI workflow and `justfile` invoked `cargo run -p runie-core --example write_config_schema --features schema`, but `runie-core` did not declare a `schema` feature. `schemars` was already a normal dependency and the schema example is ungated, so the flag was removed.

## Acceptance Criteria

- [x] Remove `--features schema` from `.github/workflows/ci.yml` schema job.
- [x] Remove `--features schema` from the `justfile` schema recipe.
- [x] Remove or update the comment in `crates/runie-core/examples/write_config_schema.rs` that references the flag.
- [x] The schema job/recipe runs successfully without the flag.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `schema_example_runs_without_feature_flag` — invoking the example without `--features schema` produces `config.schema.json`.

### Layer 2 — Event Handling
- [x] N/A.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `.github/workflows/ci.yml`
- `justfile`
- `crates/runie-core/examples/write_config_schema.rs`

## Notes

- This is a prerequisite for `replace-config-validator-with-jsonschema` because CI must be able to generate the schema reliably.
- No new dependencies are required.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
