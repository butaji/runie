# Generate config.schema.json from code

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`config.schema.json` is checked in and mirrors defaults declared in `crates/runie-core/src/config.rs` (`truncation.max_lines=2000`, `max_bytes=51200`, `telemetry.enabled=true`, `ui.vim_mode=true`, etc.). The two sources drift silently.

## Acceptance Criteria

- [ ] Schema generation from `schemars::schema_for!(Config)` becomes the single source.
- [ ] A CI check fails if `config.schema.json` does not match generated output.
- [ ] The checked-in file can be regenerated with one command (e.g., `cargo run --bin generate-schema`).
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `generated_schema_matches_checked_in` — test that reads both files and asserts equality.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `config.schema.json`
- `crates/runie-core/src/config.rs`
- `crates/runie-core/Cargo.toml`
- `.github/workflows/ci.yml`
- Possibly new schema-generation binary.

## Notes

Coordinate with `reconsider-schemars-jsonschema` if deciding to remove `schemars`/`jsonschema` entirely; in that case hand-validate a smaller schema instead.
