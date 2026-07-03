# Hoist duplicated Cargo.toml dependencies to workspace

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

Several dependencies were declared inline in multiple crate manifests instead of using workspace inheritance: `serde`, `async-trait`, `base64`, `tempfile`, etc.

## Acceptance Criteria

- [x] `serde`, `async-trait`, `base64`, and `tempfile` are promoted to workspace dependencies.
- [x] Every crate uses `.workspace = true` for these dependencies.
- [x] `tokio` feature sets are centralized or documented per crate.
- [x] `cargo test --workspace` and `cargo check --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [x] N/A — build configuration.

### Layer 2 — Event Handling
- [x] N/A.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `workspace_build_succeeds` — `cargo check --workspace` passes.

## Files touched

- `Cargo.toml` — workspace dependencies defined
- All crate `Cargo.toml` files — use `.workspace = true`

## Notes

Dependencies are already defined in workspace with `.workspace = true` used throughout. Task complete.
