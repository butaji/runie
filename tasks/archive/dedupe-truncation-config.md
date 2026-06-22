# Deduplicate truncation config struct

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`runie-core/src/config.rs:99` `TruncationSection` (serde config) and `runie-agent/src/truncate.rs:37` `TruncationConfig` are duplicate serde structs with identical fields (`max_lines`, `max_bytes`) and identical defaults (`2000`, `50*1024`). A bridge fn `policy_from_section` (truncate.rs:55) exists only because of the duplication. Two structs parse the same TOML.

## Acceptance Criteria

- [ ] `runie-agent` reuses `runie_core::config::TruncationSection` directly.
- [ ] `TruncationConfig` deleted from `truncate.rs`.
- [ ] `TruncationPolicy` (runtime view) kept; bridge fn simplified or inlined.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `truncation_policy_from_config` — `TruncationPolicy` builds from a `TruncationSection` with custom values.
- [ ] `truncation_defaults_match` — defaults still `2000` lines / `50*1024` bytes.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_long_output_truncates` — a long tool output is truncated per config after refactor.

## Files touched

- `crates/runie-agent/src/truncate.rs`
- `crates/runie-agent/src/lib.rs` (imports)

## Notes

Single source of truth for the TOML schema. The doc comment on `policy_from_section` already admits this is the wiring path.
