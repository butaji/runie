# Rename `telemetry.rs` to `tracing_init.rs`

**Status**: done
**Milestone**: R7
**Category**: Observability
**Priority**: P3

**Depends on**: extract-shared-tracing-subscriber-init
**Blocks**: none

## Description

`crates/runie-core/src/telemetry.rs` now only initializes the tracing subscriber. Rename it to avoid confusion with the old telemetry collector concept.

## Acceptance Criteria

- [ ] Rename `telemetry.rs` to `tracing_init.rs`.
- [ ] Update `pub mod telemetry` to `pub mod tracing_init`.
- [ ] Update all call sites.
- [ ] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [ ] `tracing_init_module_exists` — module renamed and exports preserved.

## Files touched

- `crates/runie-core/src/telemetry.rs`
- `crates/runie-core/src/lib.rs`
- `crates/runie-tui/src/main.rs`
- `crates/runie-cli/src/main.rs`

## Notes

- Pure rename; no behavior change.
