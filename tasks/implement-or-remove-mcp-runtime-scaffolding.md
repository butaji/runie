# Implement MCP client runtime or remove config scaffolding

**Status**: todo
**Milestone**: R5
**Category**: Architecture / Tools
**Priority**: P2

**Depends on**: delete-or-fix-dead-mcp-feature-flag
**Blocks**: none

## Description

`crates/runie-core/src/config/mcp.rs` and `crates/runie-cli/src/mcp.rs` manage MCP server configuration, but no runtime client connects to them. Either implement a stdio/SSE MCP client using `rmcp` or delete the unused config scaffolding.

## Acceptance Criteria

- [ ] Decide whether Runie will ship MCP client support in this milestone.
- [ ] If yes: implement an MCP client runtime (`crates/runie-core/src/mcp/runtime.rs`) with stdio and SSE transports using `rmcp`.
- [ ] If no: delete `config/mcp.rs`, `runie-cli/src/mcp.rs`, the `mcp` feature flag, and the `runie mcp` subcommands.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `mcp_config_roundtrip` — config serialization/deserialization works (if kept).

### Layer 2 — Event Handling
- [ ] `mcp_client_emits_tool_call` — a mock MCP server produces the expected tool-call event (if implemented).

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `mcp_stdio_client_e2e` — a provider turn invokes an MCP tool through stdio (if implemented).

## Files touched

- `crates/runie-core/src/config/mcp.rs`
- `crates/runie-core/src/config/mod.rs`
- `crates/runie-cli/src/mcp.rs`
- `crates/runie-cli/src/commands.rs`
- `crates/runie-core/Cargo.toml`

## Notes

- If MCP is removed, update `docs/Architecture.md` and `Configuration.md` to remove MCP references.
- If MCP is implemented, ensure it does not duplicate the existing built-in tool loop.
- **Update after review:** `runie-cli/src/mcp.rs` contains dead manual argv parsers behind `#[allow(dead_code)]]`; delete them as part of this task or `replace-remaining-custom-parsers-and-macros-with-strum.md`.
