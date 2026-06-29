# Implement MCP client runtime or remove config scaffolding

**Status**: done
**Milestone**: R5
**Category**: Architecture / Tools
**Priority**: P2
**Note**: Dead CLI MCP scaffolding removed; config types retained in RactorConfigActor.

**Depends on**: delete-or-fix-dead-mcp-feature-flag
**Blocks**: none

## Description

`crates/runie-cli/src/mcp.rs` and the `runie mcp` CLI subcommands managed MCP server configuration, but no runtime client connected to them. The MCP config types in `runie-core` remain useful (wired to `RactorConfigActor`), but the CLI commands are dead code.

**Decision**: Delete the dead CLI scaffolding. Keep the config types since they're used by the actor system.

## What was deleted

- `crates/runie-cli/src/mcp.rs` (dead CLI implementation)
- `Mcp` and `McpCommand` variants from `crates/runie-cli/src/main.rs`
- Dead argv-based argument parsers (previously `#[allow(dead_code)]`)

## What stays

- `crates/runie-core/src/config/mcp.rs` — MCP server config types used by `RactorConfigActor`
- `rmcp` dependency — used by `runie-core/src/tool/schema.rs` for MCP tool schema generation

## Future work

If MCP client runtime is needed, it would require:
- A client-side MCP library (current `rmcp` is server-focused)
- Integration with the tool execution pipeline in `runie-agent`

## Acceptance Criteria

- [x] `runie mcp` CLI command is removed from `runie-cli`.
- [x] `crates/runie-cli/src/mcp.rs` is deleted.
- [x] `RactorConfigActor` MCP methods remain (they're wired to config).
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `mcp_config_roundtrip` — config serialization/deserialization still works.

### Layer 2 — Event Handling
- N/A — CLI removal doesn't affect event handling.

### Layer 4 — Provider Replay / Mock-Tool E2E
- N/A.
