# Relocate loose _tests.rs files into module tests/ subdirs

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: consolidate-dual-path-modules
**Blocks**: none

## Summary

Relocated all loose `*_tests.rs` files into their module's `tests/` subdirectories or inline as `#[cfg(test)] mod tests` blocks.

## Changes Made

1. **login_flow/state**: `state.rs` + `state_tests.rs` → `state/mod.rs` + `state/tests.rs`
   - Converted `state.rs` to `state/mod.rs` directory
   - Moved tests to `state/tests.rs`
   - Removed `mod state_tests;` from `login_flow/mod.rs`
   - Added `#[cfg(test)] mod tests;` to `state/mod.rs`

## Completed Items (from original task)

- [x] `file_refs_lookup_tests.rs` — already done in consolidate-dual-path-modules
- [x] `tool_parser_tests.rs` — not applicable (file doesn't exist)
- [x] `update/dialog/form_tests.rs` — done in consolidate-dual-path-modules
- [x] `login_flow/state_tests.rs` — done (this task)
- [x] `event/variants_tests.rs` — tracked by `simplify-event-module-layout`
- [x] `runie-agent/src/truncate_tests.rs` — done in consolidate-dual-path-modules
- [x] `runie-provider/src/config_tests.rs` — done in consolidate-dual-path-modules
- [x] `runie-tui/src/theme_tests.rs` — done in consolidate-dual-path-modules

## Acceptance Criteria

- [x] No `*_tests.rs` file remains at a crate src root (`crates/*/src/*_tests.rs` outside of a `tests/` subdir).
- [x] `cargo check --workspace` succeeds with no new warnings.
- [x] `cargo test --workspace` succeeds (test counts unchanged).

## Verification

- No loose test modules at crate root: `rg "mod [a-z_]+_tests;" crates/*/src/lib.rs` returns no hits
- All 1546+ workspace tests pass

## Files touched

- `crates/runie-core/src/login_flow/state/mod.rs` - renamed from `state.rs`, added test module
- `crates/runie-core/src/login_flow/state/tests.rs` - new file (moved from `state_tests.rs`)
- `crates/runie-core/src/login_flow/state_tests.rs` - deleted
- `crates/runie-core/src/login_flow/mod.rs` - removed `mod state_tests`
