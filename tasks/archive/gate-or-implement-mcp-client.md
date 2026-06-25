# Gate or implement MCP client

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/mcp.rs` (483 LOC) is a dead code block. It is:
- Only compiled behind the `#[cfg(feature = "mcp")]` feature flag.
- Not imported by any other module in the workspace (zero external consumers).
- Conditionally included in `runie-core/src/lib.rs` with `#[cfg(feature = "mcp")]pub mod mcp;`.

The module defines `McpServerConfig`, `McpConnectionStatus`, and related types but has no live implementation that connects to a real MCP server.

Decision: **delete** (YAGNI — MCP client is not implemented; `mcp.rs` is scaffolding that was never wired up).

## Acceptance criteria

- [x] Decision made: **delete** (YAGNI — MCP client not implemented; mcp.rs was dead scaffolding behind a feature flag).
- [x] `crates/runie-core/src/mcp.rs` deleted; `#[cfg(feature = "mcp")]pub mod mcp;` removed from `lib.rs`.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- N/A.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [x] `workspace_builds_after_mcp_decision` — `cargo check --workspace` succeeds.

## Files touched

- `crates/runie-core/src/mcp.rs` (likely deleted)
- `crates/runie-core/src/lib.rs` (remove conditional module export)

## Notes

The MCP (Model Context Protocol) module provides types but no actual MCP client implementation. If MCP support is needed in the future, it should be implemented as a proper `McpActor` with a real client library, not shipped as dead scaffolding behind a feature flag.
