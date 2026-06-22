# Deduplicate fresh_state test helper

**Status**: done
**Milestone**: R4
**Category**: Architecture / Testing
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`fn fresh_state() -> AppState` was duplicated ~36× across test files. Shared `pub fn fresh_state` existed in `slash.rs`/`safety.rs` but siblings didn't reuse them. Companion `fn type_str(state, text)` was byte-identical in multiple files.

## Changes Made

- Created `runie-testing/src/state.rs` with shared `fresh_state()` and `type_str()` for `runie-tui` tests.
- Created `crate::tests::fresh_state()` and `crate::tests::type_str()` in `runie-core/src/tests/mod.rs` (cannot use `runie-testing` due to circular dependency during test compilation).
- Updated ~36 test files to import from shared helpers instead of defining locally.
- Added `runie-testing` as dev-dependency to `runie-tui`.

## Limitations

Due to circular dependency (`runie-testing` → `runie-core` → `runie-testing` in test context), two definitions exist:
- `runie-testing/src/state.rs` for `runie-tui` tests
- `runie-core/src/tests/mod.rs` for `runie-core` tests

This is architecturally sound and all tests pass.

## Acceptance Criteria

- [x] A single shared test-support module exposes `pub fn fresh_state()` and `pub fn type_str(state, text)` (extend `runie-testing` for runie-tui, `#[cfg(test)] mod` for runie-core).
- [x] All ~36 local `fresh_state` copies replaced with the shared import.
- [x] All `type_str` copies replaced with the shared import.
- [x] `cargo test -p runie-core --lib` succeeds (1331 tests).
- [x] `cargo test -p runie-tui --lib` succeeds (681 tests).

## Tests

### Layer 1 — State/Logic
- [x] `shared_type_str_appends` — `type_str` produces the expected input buffer content.

### Layer 2 — Event Handling
- N/A — test helper only.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- N/A.

## Files touched

- `crates/runie-testing/src/lib.rs` - added `state` module export
- `crates/runie-testing/src/state.rs` - new shared helpers for runie-tui
- `crates/runie-core/src/tests/mod.rs` - shared helpers for runie-core
- `crates/runie-tui/Cargo.toml` - added runie-testing dev-dependency
- ~36 test files under `crates/runie-core/src/tests/` and `crates/runie-tui/src/tests/core/`

## Notes

Successfully deduplicated ~36 local `fresh_state` and `type_str` definitions. The circular dependency between `runie-core` and `runie-testing` during test compilation required maintaining two definitions, but both are used consistently within their respective crate test contexts.
