# Hoist duplicated Cargo.toml dependencies to workspace

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

Several dependencies are declared inline in multiple crate manifests instead of using workspace inheritance: `serde = { version = "1.0", features = ["derive"] }` in 5 crates, `async-trait = "0.1"` in 3 crates, `base64 = "0.22"` in 2 crates, `tempfile = "3"` in 3 dev-dependencies, and `tokio` feature sets redeclared per crate. This causes silent feature-set drift and makes upgrades error-prone.

## Acceptance Criteria

- [ ] `serde`, `async-trait`, `base64`, and `tempfile` are promoted to workspace dependencies.
- [ ] Every crate uses `.workspace = true` for these dependencies.
- [ ] `tokio` feature sets are centralized or documented per crate.
- [ ] `cargo test --workspace` and `cargo check --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [ ] N/A — build configuration.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `workspace_build_succeeds` — `cargo check --workspace` passes.

## Files touched

- `Cargo.toml`
- `crates/runie-core/Cargo.toml`
- `crates/runie-agent/Cargo.toml`
- `crates/runie-provider/Cargo.toml`
- `crates/runie-engine/Cargo.toml`
- `crates/runie-tui/Cargo.toml`
- `crates/runie-server/Cargo.toml`
- `crates/runie-json/Cargo.toml`
- `crates/runie-print/Cargo.toml`
- `crates/runie-testing/Cargo.toml`

## Notes

Coordinate with `drop-thiserror-and-async-trait-deps` if removing `async-trait` entirely. Also align with `fix-duplicate-cargo-toml-keys` for the duplicate-key cleanup already planned.
