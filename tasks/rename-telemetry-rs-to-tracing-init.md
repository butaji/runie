# Rename `telemetry.rs` to `tracing_init.rs`

**Status**: done
**Note**: Verified 2026-06-29 — `tracing_init.rs` exists and `telemetry.rs` does not exist.
**Milestone**: R7
**Category**: Observability
**Priority**: P3

**Depends on**: extract-shared-tracing-subscriber-init
**Blocks**: none

## Description

`crates/runie-core/src/telemetry.rs` now only initializes the tracing subscriber. Rename it to avoid confusion with the old telemetry collector concept.

## Acceptance Criteria

- [x] Rename `telemetry.rs` to `tracing_init.rs`.
- [x] Update `pub mod telemetry` to `pub mod tracing_init`.
- [x] Update all call sites.
- [x] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [x] `tracing_init_module_exists` — module renamed and exports preserved.

## Files touched

- `crates/runie-core/src/telemetry.rs`
- `crates/runie-core/src/lib.rs`
- `crates/runie-tui/src/main.rs`
- `crates/runie-cli/src/main.rs`

## Notes

- Pure rename; no behavior change.
