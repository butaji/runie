# Delete leftover `telemetry.rs` stub

**Status**: todo
**Milestone**: R6
**Category": Observability
**Priority": P3

**Depends on**: replace-custom-telemetry-with-tracing-layer
**Blocks**: none

## Description

`crates/runie-core/src/telemetry.rs` now contains only two trivial tests asserting `TelemetrySection::default().enabled` is true. Actual telemetry is emitted via `tracing::info!`. Move the tests into `config/tests` and remove `pub mod telemetry` from `lib.rs`.

## Acceptance Criteria

- [ ] Move the two tests to an appropriate `config` test module.
- [ ] Delete `crates/runie-core/src/telemetry.rs`.
- [ ] Remove `pub mod telemetry` from `crates/runie-core/src/lib.rs`.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `telemetry_section_default_enabled` — test survives in new location.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/telemetry.rs`
- `crates/runie-core/src/lib.rs`
- `crates/runie-core/src/config/mod.rs` or `config/tests.rs`

## Notes

- If `extract-shared-tracing-subscriber-init.md` repopulates `telemetry.rs`, keep the module and delete only the old stub code.
