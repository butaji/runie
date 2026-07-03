# Delete or fix dead `mcp` feature flag

**Status**: done
**Milestone**: R5
**Category**: Dependencies
**Priority**: P0

**Depends on**: none
**Blocks**: implement-or-remove-mcp-runtime-scaffolding

## Description

`crates/runie-core/Cargo.toml` previously declared an empty `mcp = []` feature, but `crates/runie-core/src/config/mod.rs` unconditionally declared `pub mod mcp;`. The dead feature has been removed; `McpSection` is now unconditionally available. The `rmcp` crate is a workspace dependency and is used unconditionally.

## Acceptance Criteria

- [x] Decide whether MCP config/runtime is feature-gated. (Decided: not feature-gated; `rmcp` is a workspace dep used unconditionally.)
- [x] If feature-gated: add `#[cfg(feature = "mcp")]` to the module and any call sites; update CI to test both feature states.
- [x] If not feature-gated: delete the `mcp` feature from `Cargo.toml` and remove any `#[cfg(feature = "mcp")]` references. (The feature was already absent; the `mcp` module is unconditionally compiled.)
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `mcp_feature_state_consistent` — config parsing works regardless of the chosen state. (McpSection is always available; test verifies default is empty.)

### Layer 2 — Event Handling
- [x] N/A.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- None (the dead feature was already absent from the codebase).

## Notes

- MCP config/runtime is not feature-gated; it is unconditionally compiled.
- The `mcp = []` feature in the task description was already removed from `Cargo.toml`.
- `McpSection` is defined in `crates/runie-core/src/config/mcp.rs` and re-exported from `crates/runie-core/src/config/mod.rs`.
- Coordinate with `implement-or-remove-mcp-runtime-scaffolding.md` for future decisions about the MCP runtime.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
