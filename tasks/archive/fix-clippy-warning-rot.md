# Fix Clippy Warning Rot

**Status**: done
**Milestone**: R3
**Category**: Core / State
**Priority**: P1

**Depends on**: (none)
**Blocks**: (none)

## Description

`cargo clippy --workspace` emits 233 warnings, 126 from `runie-core` alone. Categories include unused imports, dead functions, dead types/fields, and unused variables. Noise hides real bugs.

## Acceptance Criteria

- [x] Workspace clippy warnings are reduced from 233 to 0.
- [x] Dead code is deleted or explicitly allowed with a comment.
- [x] Unused imports are removed.
- [x] Intentionally kept stubs are documented or gated.

## Tests

### Layer 1 — State/Logic
- [x] `clippy_count_below_baseline` — `cargo clippy --workspace` warning count is 0.

### Layer 2 — Event Handling
- [x] `cargo_test_passes` — `cargo test --workspace` passes (332 tests + doc tests).

## Files touched

- Workspace-wide; concentrated in `crates/runie-core/src/`

## Notes

Ran `cargo clippy --fix --workspace` as a first pass and reviewed all automated changes. Restored imports needed only in tests with `#[cfg(test)]`, fixed safe manual warnings (unused variables, unused imports, needless struct updates, `while_let_loop`, `ptr_arg`, etc.), and added `#[allow(...)]` with comments for intentionally kept stubs/large enums/complex types. Final `cargo clippy --workspace` reports 0 warnings; `cargo test --workspace` is green.
