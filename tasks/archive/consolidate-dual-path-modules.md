# Consolidate Dual-Path Modules

**Status**: done
**Milestone**: R4
**Category**: Architecture / Refactoring
**Priority**: P2

**Depends on**: event-taxonomy-for-actor-state-sync
**Blocks**: relocate-loose-tests-files

**Progress**: Moved `file_refs_lookup_tests.rs` from src root to `tests/file_refs_lookup.rs`; declared in `tests/mod.rs`.

## Summary

Consolidated modules that existed in both `src/` and `tests/` directories into a single canonical location. Removed all `#[path = "..."]` workarounds and loose `*_tests.rs` files at crate root level.

## Changes Made

1. **runie-tui/theme**: `theme_tests.rs` → `theme/tests.rs`
   - Deleted loose `theme_tests.rs` from crate root
   - Created `theme/tests.rs` with test content
   - Added `#[cfg(test)] mod tests;` to `theme/mod.rs`

2. **runie-agent/truncate**: `truncate.rs` + `truncate_tests.rs` → `truncate/mod.rs` + `truncate/tests.rs`
   - Converted `truncate.rs` to `truncate/mod.rs` directory
   - Moved tests to `truncate/tests.rs`
   - Added `#[cfg(test)] mod tests;` to `truncate/mod.rs`
   - Deleted loose `truncate_tests.rs` from crate root

3. **runie-provider/config**: `config.rs` + `config_tests.rs` → `config/mod.rs` + `config/tests.rs`
   - Converted `config.rs` to `config/mod.rs` directory
   - Moved tests to `config/tests.rs`
   - Deleted loose `config_tests.rs` from crate root

4. **runie-core/update/agent/core**: Removed `#[path = "core/tests.rs"]` workaround
   - Converted `core.rs` to `core/mod.rs` directory
   - Removed `#[path]` directive
   - Added `#[cfg(test)] mod tests;` to `core/mod.rs`

5. **runie-core/update/dialog/form**: Removed `#[path = "form_tests.rs"]` workaround
   - Converted `form.rs` to `form/mod.rs` directory
   - Moved `form_tests.rs` to `form/tests.rs`
   - Removed `#[path]` directive
   - Added `#[cfg(test)] mod tests;` to `form/mod.rs`

## Acceptance Criteria

- [x] No duplicate module definitions
- [x] Tests import from canonical location
- [x] `cargo test --workspace` passes

## Verification

- No `#[path = "..."]` workarounds remain: `rg "#\[path = " crates/ --type rust` returns no hits
- No loose `*_tests.rs` files at crate root level
- All 1546+ workspace tests pass

## Files touched

- `crates/runie-tui/src/theme/mod.rs` - added test module
- `crates/runie-tui/src/theme/tests.rs` - new file (moved from `theme_tests.rs`)
- `crates/runie-tui/src/theme_tests.rs` - deleted
- `crates/runie-tui/src/lib.rs` - removed `mod theme_tests`
- `crates/runie-agent/src/truncate/mod.rs` - renamed from `truncate.rs`
- `crates/runie-agent/src/truncate/tests.rs` - new file (moved from `truncate_tests.rs`)
- `crates/runie-agent/src/truncate_tests.rs` - deleted
- `crates/runie-agent/src/lib.rs` - removed `mod truncate_tests`
- `crates/runie-provider/src/config/mod.rs` - renamed from `config.rs`
- `crates/runie-provider/src/config/tests.rs` - new file (moved from `config_tests.rs`)
- `crates/runie-provider/src/config_tests.rs` - deleted
- `crates/runie-provider/src/lib.rs` - removed `mod config_tests`
- `crates/runie-core/src/update/agent/core/mod.rs` - renamed from `core.rs`, removed `#[path]`
- `crates/runie-core/src/update/agent/core/tests.rs` - already existed
- `crates/runie-core/src/update/dialog/form/mod.rs` - renamed from `form.rs`, removed `#[path]`
- `crates/runie-core/src/update/dialog/form/tests.rs` - new file (moved from `form_tests.rs`)
- `crates/runie-core/src/update/dialog/form_tests.rs` - deleted
