# Use workspace deps in runie-testing Cargo.toml

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P3

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-testing/Cargo.toml` uses `runie-core = { path = "../runie-core" }` for internal crates, while every other crate uses `runie-core.workspace = true`. This inconsistency bypasses workspace version management.

## Acceptance Criteria

- [ ] All internal crate dependencies in `runie-testing/Cargo.toml` use `.workspace = true`.
- [ ] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [ ] N/A.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `workspace_build_succeeds` — `cargo check --workspace` passes.

## Files touched

- `crates/runie-testing/Cargo.toml`

## Notes

Combine with `hoist-cargo-workspace-deps` if doing a broader Cargo.toml cleanup pass.
