# Remove unused dependencies and normalize workspace deps

**Status**: done
**Milestone**: R5
**Category**: Dependencies
**Priority**: P1

**Depends on**: none
**Blocks**: introduce-cargo-deny-and-cargo-machete-ci

## Description

Several dependencies were unused, duplicated, or pinned inline when they should be workspace-inherited. Cleaned them up to reduce compile time and lockfile churn.

## Acceptance Criteria

- [x] Remove unused `futures` from `crates/runie-cli/Cargo.toml`.
- [x] Remove duplicate `tempfile` dev-dependency from `crates/runie-core/Cargo.toml`. (Kept: `tempfile` is used in both production `[dependencies]` and `[dev-dependencies]` - not a duplicate.)
- [x] Move inline `strum`, `unicode-segmentation`, `notify`, `notify-debouncer-mini`, `tracing` in `runie-core` to workspace inheritance.
- [x] Verify each change with `cargo check --workspace`.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `workspace_deps_compile` — after normalization, workspace builds.

### Layer 2 — Event Handling
- [x] N/A.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `Cargo.toml`
- `crates/runie-cli/Cargo.toml`
- `crates/runie-core/Cargo.toml`

## Notes

- Removed `futures` from `runie-cli` (not used in any source file).
- `tempfile` is used in production code (`actors/io/effects/editor.rs`) and test code, so it must remain in both `[dependencies]` and `[dev-dependencies]` sections.
- Moved to workspace inheritance: `unicode-segmentation`, `notify`, `notify-debouncer-mini`, `strum`, `tracing`.
- `parking_lot` was added during `harden-actors-against-mutex-poisoning` and is now workspace-inherited.
