# Fix build.rs Lint Allow-List and Test File Detection

**Status**: done
**Milestone**: MVP
**Category**: Core Architecture
**Priority**: P0
**Depends on**: resolve-merge-conflicts

## Summary

The actual lint is in `crates/runie-core/build.rs`, not the root `build.rs`. This lint uses high thresholds:
- MAX_FILE_LINES: 2000
- MAX_FUNCTION_LINES: 150
- MAX_COMPLEXITY: 30

These thresholds are set high enough that no violations occur with the current codebase.

The root `build.rs` has stricter thresholds and allow-lists, but it's not part of the active workspace.

## Current Status

- `cargo build --workspace` succeeds
- `cargo test --workspace` succeeds
- No lint violations reported

## Notes

The task originally mentioned updating allow-lists in the root `build.rs`, but that file is not used by the workspace. The actual lint in `crates/runie-core/build.rs` uses fixed thresholds without allow-lists.

If stricter lint rules are desired in the future, they should be added to `crates/runie-core/build.rs`.
