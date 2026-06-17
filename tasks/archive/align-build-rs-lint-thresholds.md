# Align Build.rs Lint Thresholds (Strict Enforcement)

**Status**: done
**Milestone**: R4
**Category**: Architecture / Refactoring
**Priority**: P0

**Depends on**: (all simplification tasks)
**Blocks**: (none)

## Description

`crates/runie-core/build.rs` enforces strict lint thresholds.

| Threshold | Value | Scope |
|-----------|-------|-------|
| File lines | 500 | All source files |
| Function lines | 40 | Production code only |
| Complexity | 10 | Production code only |

Test functions and files under `tests/` directories are exempt from function-length and complexity checks so tests can remain comprehensive. File-length enforcement applies to every `.rs` file.

## Current Violations

None. `cargo build --workspace` succeeds with zero violations.

## Acceptance Criteria

- [x] `ALLOWED_FILE_VIOLATIONS` removed from `build.rs` ✓
- [x] `ALLOWED_FUNC_VIOLATIONS` removed from `build.rs` ✓
- [x] `is_allowed_func()` and related allow-list logic removed ✓
- [x] File-length violations cause `cargo build` to fail ✓
- [x] Function-length/complexity violations cause `cargo build` to fail for production code ✓
- [x] Tests are exempt from function-length/complexity checks ✓
- [x] `cargo build --workspace` succeeds with no violations ✓
- [x] `cargo test --workspace` succeeds ✓
- [x] `cargo clippy --workspace -- -D warnings` succeeds ✓

## Files touched

- `crates/runie-core/build.rs`

## Notes

Enforcement is active. Any future production function longer than 40 lines or with complexity over 10 will fail the workspace build.
