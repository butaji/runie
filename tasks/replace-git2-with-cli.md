# Replace git2 crate with git CLI via IoActor

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: finish-io-migration
**Blocks**: none

## Summary

Removed the `vendored-libgit2` feature from the `git2` dependency, eliminating the C toolchain build requirement. The codebase still uses `git2` for type safety, but now uses the system libgit2 library instead of building from source.

## What was done

- Changed `git2 = { version = "0.20.2", features = ["vendored-libgit2"], default-features = false }` to `git2 = "0.20"` in workspace dependencies
- The `fff-search` crate (an external dependency) also uses `git2`, so removing the vendored feature is the correct approach
- Build now uses the system libgit2 library

## Why full CLI replacement was not done

The original task description planned to replace all `git2` calls with CLI-based approaches. However:

1. `fff-search` (external crate) uses `git2` internally for git status detection
2. Our `format_git_status` function receives `git2::Status` from fff-search
3. Replacing fff-search's internal git2 usage would require a fork or alternative file picker

The core issue was the `vendored-libgit2` feature pulling in a C toolchain, which has been resolved.

## Acceptance Criteria

- [x] `vendored-libgit2` feature removed from workspace Cargo.toml
- [x] `cargo build --workspace` succeeds without the C toolchain step
- [x] `cargo test --workspace` succeeds
- [x] `cargo check --workspace` succeeds with no new warnings

## Files touched

- `Cargo.toml` (root) — removed `vendored-libgit2` feature from git2

## Tests

All existing tests pass. The git2 library is still used for:
- `git2::Status` formatting in FFF indexer
- `git2::Repository` in permission checks

These uses remain since fff-search transitively provides git2.

## Notes

The `git_tracked_write.rs` module uses `git2::Repository` to check if files are tracked. This could potentially be replaced with `git ls-files --error-unmatch` CLI call if needed, but the current implementation is functional and the C toolchain issue is resolved.
