# Fix build.rs Linter False Negatives

**Status**: done
**Completed**: 2026-06-14
**Milestone**: R3
**Category**: Configuration
**Priority**: P0

## Description

The workspace lint script at `crates/runie-core/build.rs` enforces 500 lines/file, 40 lines/function, and 10 complexity. It currently only detects functions that start with exactly `fn `, so it misses `pub fn`, `pub(crate) fn`, `async fn`, `const fn`, `unsafe fn`, etc. It also under-counts complexity (missing `else if`, `&&`, `||`, `?`).

## Acceptance Criteria

- [ ] Function detection matches `fn ` optionally preceded by visibility and modifiers (`pub`, `pub(crate)`, `async`, `const`, `unsafe`, `static`).
- [ ] Complexity counting includes `else if`, `&&`, `||`, `?`, and loops.
- [ ] Existing violations that were hidden are fixed or the limits are relaxed/documented.
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `linter_catches_pub_fn`.
- [ ] `linter_catches_async_fn`.
- [ ] `linter_counts_else_if_and_short_circuit`.

### Layer 4 — Smoke
- [ ] A deliberately oversized `pub fn` in a test fixture is rejected.
