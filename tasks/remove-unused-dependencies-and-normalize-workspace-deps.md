# Remove unused dependencies and normalize workspace deps

**Status**: todo
**Milestone**: R5
**Category**: Dependencies
**Priority**: P1

**Depends on**: none
**Blocks**: introduce-cargo-deny-and-cargo-machete-ci

## Description

Several dependencies are unused, duplicated, or pinned inline when they should be workspace-inherited. Clean them up to reduce compile time and lockfile churn.

## Acceptance Criteria

- [ ] Remove unused `futures` from `crates/runie-cli/Cargo.toml`.
- [ ] Remove duplicate `tempfile` dev-dependency from `crates/runie-core/Cargo.toml`.
- [ ] Move inline `strum`, `unicode-segmentation`, `notify`, `notify-debouncer-mini`, `tracing` in `runie-core` to workspace inheritance.
- [ ] Verify each change with `cargo check --workspace`.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `workspace_deps_compile` — after normalization, workspace builds.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `Cargo.toml`
- `crates/runie-cli/Cargo.toml`
- `crates/runie-core/Cargo.toml`

## Notes

- Run `cargo tree -d` afterward to document remaining duplicate transitive versions for `introduce-cargo-deny-and-cargo-machete-ci.md`.
- Do not remove `futures` from other crates where it is actually used.
