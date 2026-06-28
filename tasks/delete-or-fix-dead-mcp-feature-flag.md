# Delete or fix dead `mcp` feature flag

**Status**: done
**Milestone**: R5
**Category**: Dependencies
**Priority**: P0

**Depends on**: none
**Blocks**: implement-or-remove-mcp-runtime-scaffolding

## Description

`crates/runie-core/Cargo.toml` declares an empty `mcp = []` feature, but `crates/runie-core/src/config/mod.rs` unconditionally declares `pub mod mcp;`. Either the module should be gated behind the feature or the feature should be deleted.

## Acceptance Criteria

- [ ] Decide whether MCP config/runtime is feature-gated.
- [ ] If feature-gated: add `#[cfg(feature = "mcp")]` to the module and any call sites; update CI to test both feature states.
- [ ] If not feature-gated: delete the `mcp` feature from `Cargo.toml` and remove any `#[cfg(feature = "mcp")]` references.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `mcp_feature_state_consistent` — config parsing works regardless of the chosen state.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/Cargo.toml`
- `crates/runie-core/src/config/mod.rs`
- `crates/runie-core/src/config/mcp.rs`

## Notes

- Coordinate with `implement-or-remove-mcp-runtime-scaffolding.md`: if MCP runtime is removed, the feature should also be deleted.
