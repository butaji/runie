# Standardize test layout on inline mod tests

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: relocate-loose-tests-files
**Blocks**: none

## Description

Three test layouts coexist in the workspace:

| Layout | Count | Example |
|--------|-------|---------|
| Inline `#[cfg(test)] mod tests` | 125 | Idiomatic Rust |
| Sibling `*_tests.rs` / `tests.rs` files | 29 | `foo_tests.rs` next to `foo.rs` |
| `tests/` directories | 9 | `crates/runie-core/tests/` |

The sibling-file and `tests/`-dir layouts fragment test discovery and require manual `mod` wiring. The idiomatic Rust convention is inline `#[cfg(test)] mod tests` in the same file as the code under test. Standardize on that, with `tests/` dirs reserved for integration tests only (e.g. `arch_guardrails.rs` which scans source files).

## Acceptance Criteria

- [ ] Audit complete: list all 29 `*_tests.rs`/`tests.rs` sibling files and 9 `tests/` dirs.
- [ ] Sibling `*_tests.rs` files converted to inline `#[cfg(test)] mod tests` in their parent module (or moved into a `tests/` dir if they are integration tests).
- [ ] `tests/` dirs retained only for true integration tests (multi-module, external-crate boundary). Document which ones qualify.
- [ ] `relocate-loose-tests-files` and `dedupe-fresh-state-test-helper` satisfied by this standardization.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `no_sibling_test_files_outside_tests_dirs` — grep assertion: no `*_tests.rs` files at `src/` level outside `tests/` directories.
- [ ] `inline_mod_tests_present` — each converted module has `#[cfg(test)] mod tests` with at least one `#[test]`.

### Layer 2 — Event Handling
- [ ] N/A — test layout, not event logic.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Smoke / Crash
- [ ] `cargo test --workspace` green confirms all tests still run after conversion.

## Files touched

- 29 `*_tests.rs`/`tests.rs` sibling files → inline or move to `tests/`
- 9 `tests/` dirs → audit, keep integration tests, inline the rest
- Parent `mod.rs` files — update `#[cfg(test)] mod tests;` declarations

## Notes

Depends on `relocate-loose-tests-files` (which moves some loose test files) — do that first, then this task standardizes the rest. The 40-line function limit is exempt for test functions (per `AGENTS.md`), so inlining large test modules is fine. Exception: `crates/runie-core/tests/arch_guardrails.rs` stays as an integration test because it scans source files via `std::fs` (not a unit test). `consolidate-login-logout-tests` and `dedupe-test-gate-and-provider-helpers` are sub-tasks absorbed by this standardization.
