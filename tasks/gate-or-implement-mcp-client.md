# Gate or implement MCP client

**Status**: todo
**Milestone**: R4
**Category**: Tools
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/mcp.rs` (482 LOC) is mostly stub. `call_tool` returns hardcoded `Ok(json!({"success": true, "message": "MCP tool call not yet implemented"}))` (mcp.rs:272). `connect` spawns the child with piped stdin/stdout but never reads/writes the pipes; `McpServerHandle.child` is only killed/waited on disconnect. `tools: Vec<McpTool>` is never populated so `list_tools()` always returns empty. `JsonRpc`, `JsonRpcError`, `next_id` are all `#[allow(dead_code)]`. Only config-parse/badge/env-injection helpers genuinely work. Either implement the transport or gate the scaffolding behind a feature flag and drop the dead types.

## Acceptance Criteria

- [ ] Decision made and executed: EITHER
  - (a) MCP `call_tool` and tool discovery implemented over JSON-RPC stdio, `tools` populated from `tools/list`, `JsonRpc`/`next_id` used; OR
  - (b) `McpClientManager` reduced to config-only (parse + env + badge), dead `JsonRpc`/`JsonRpcError`/`next_id`/`call_tool` stub deleted, module gated behind `#[cfg(feature = "mcp")]` until ready.
- [ ] No `#[allow(dead_code)]` remains on MCP types.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `mcp_config_parse_works` — config parsing + env injection still works (already does).
- [ ] (option a) `mcp_call_tool_routes_to_server` — a tool call reaches the spawned server; (option b) N/A.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_mcp_connect_disconnect_no_panic` — connect/disconnect cycle does not panic or leak the child.

## Files touched

- `crates/runie-core/src/mcp.rs`
- `crates/runie-core/src/lib.rs` (feature gate if option b)

## Notes

Recommended: option (b) now. MCP is not shipping; ~50 lines of scaffolding suggesting it works is worse than an honest absence. Reintroduce behind a feature flag when actually implementing.
