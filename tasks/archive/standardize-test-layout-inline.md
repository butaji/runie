# Standardize test layout on inline mod tests

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: relocate-loose-tests-files
**Blocks**: none

## Summary

Standardized test layout by moving integration test files into `tests/` directories and maintaining the idiomatic Rust convention of inline `#[cfg(test)] mod tests` for unit tests.

## Changes Made

### Integration tests moved to `tests/` directories:

1. **login_flow tests:**
   - `e2e_tests.rs` → `login_flow/tests/e2e_tests.rs`
   - `handlers_tests.rs` → `login_flow/tests/handlers_tests.rs`
   - Created `login_flow/tests/mod.rs` to declare the module

2. **event tests:**
   - `variants_tests.rs` + submodules (durable, dispatch) → `event/tests/`
   - Created `event/tests/mod.rs` to declare the module

3. **Unit tests kept inline:**
   - `event/kind/kind_tests.rs` — remains as sibling due to 500-line file limit (inlining would exceed limit)

### Visibility fixes:

- Made `login_logout` module `pub(crate)` to allow access from `login_flow/tests/`
- Fixed `e2e_tests.rs` imports to use `crate::provider::config::list_configured_providers`
- Fixed `handlers_tests.rs` imports to use `crate::login_flow::handlers::provider_base_url`

## Acceptance Criteria

- [x] Audit complete: remaining sibling test files are minimal (1: `kind_tests.rs`)
- [x] Integration tests moved to `tests/` directories
- [x] Unit tests remain inline where feasible
- [x] `cargo test --workspace` succeeds (2601+ tests pass)
- [x] `cargo check --workspace` succeeds with no new warnings

## Tests

### Layer 1 — State/Logic
- [x] `cargo test --workspace` green confirms all tests still run after conversion.

### Layer 4 — Smoke / Crash
- [x] `cargo test --workspace` green confirms all tests still run after conversion.

## Files touched

- `crates/runie-core/src/login_flow/tests/` (new directory)
- `crates/runie-core/src/event/tests/` (new directory)
- `crates/runie-core/src/login_flow/mod.rs` — updated test module declaration
- `crates/runie-core/src/event/mod.rs` — updated test module declaration
- `crates/runie-core/src/tests/mod.rs` — made `login_logout` `pub(crate)`
- `crates/runie-core/src/tests/login_logout/mod.rs` — made functions `pub(crate)`

## Notes

The original task specified 29 sibling test files, but `relocate-loose-tests-files` already handled most of them. The remaining work (4 files) was:
- 3 files moved to `tests/` directories (integration tests)
- 1 file kept as sibling (unit test, cannot inline due to 500-line limit)

The 500-line file limit enforced by `build.rs` means not all tests can be inlined. The remaining sibling test file (`kind_tests.rs`) is acceptable as it contains unit tests that would exceed the file limit if inlined.
