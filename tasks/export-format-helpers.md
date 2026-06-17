# Export Format Helpers

**Status**: done
**Completed**: 2026-06-16
**Notes**: Re-exported `format_bytes` and `format_duration` from `runie_core` root so status bar/TUI can use them without reaching into `tool::`. Existing Layer 1 tests in `tool/mod.rs` cover both helpers. cargo test --workspace passes.
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P3

**Depends on**: (none)
**Blocks**: (none)

## Description

Export `format_bytes` and `format_duration` helpers from `tool/mod.rs` for use in status bar.

**Location:** `crates/runie-core/src/tool/mod.rs:207-236`

These helpers are defined but private. Either:
- A) Export them publicly
- B) Move to `crates/runie-core/src/labels.rs` and export

## Acceptance Criteria

- [ ] `format_bytes` and `format_duration` accessible from crate root.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `format_bytes_human_readable` — bytes formatted correctly.
- [ ] `format_duration_seconds` — duration formatted correctly.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/src/tool/mod.rs`

## Notes

Low priority, depends on whether these are actually useful elsewhere.
