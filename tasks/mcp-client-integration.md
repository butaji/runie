# MCP Client Integration

**Status**: todo
**Milestone**: R3
**Category**: Tools
**Priority**: P1

**Depends on**: tool-registry-trait
**Blocks**: mcp-servers-support

## Description

The existing `mcp-servers-support.md` task only builds the management UI
(status panel, config parsing). This task implements the actual Model Context
Protocol client so MCP servers expose real tools to the agent.

We follow Goose (`ExtensionManager` via MCP), OpenHarness (`McpClientManager`),
and Gemini CLI (`mcp-client-manager`) patterns: a manager owns server
lifecycles, discovers tools/resources, and exposes them through the same
`ToolRegistry` trait as built-ins.

## Acceptance Criteria

- [ ] `crates/runie-core/src/mcp.rs` defines:
  - `McpServerConfig { name, command, args, env }`
  - `McpStatus { Connected, Disconnected, Unavailable }`
  - `McpClientManager` with `connect`, `disconnect`, `list_statuses`,
    `list_tools`, `call_tool`
- [ ] MCP transport support for stdio servers (SSE/HTTP can be stubbed for
  future extension).
- [ ] Tool names are namespaced: `<server_name>__<tool_name>` (double
  underscore to avoid collisions).
- [ ] `McpClientManager` injects `RUNIE_MCP_<NAME>_TOKEN` env vars for managed
  credentials.
- [ ] MCP tools are registered in the same `ToolRegistry` as built-ins.
- [ ] `AgentActor` includes MCP tools in the LLM tool list when a server is
  connected.
- [ ] Connection status is published as `McpStatusChanged` events on the bus.
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `mcp_config_parses_servers` — TOML `[[mcp.servers]]` parses correctly.
- [ ] `mcp_status_from_connection` — connected/disconnected states map to
  `McpStatus`.
- [ ] `mcp_tool_name_is_namespaced` — `linear/create_issue` → `linear__create_issue`.
- [ ] `mcp_env_var_injected` — `RUNIE_MCP_LINEAR_TOKEN` is present for the
  server process.

### Layer 2 — Event Handling
- [ ] `mcp_status_changed_event_published` — connection change emits bus event.
- [ ] `mcp_tool_call_routed_to_server` — calling an MCP tool sends the correct
  JSON-RPC request.

### Layer 3 — Rendering
- [ ] `mcp_panel_shows_connected_tools` — MCP tab lists tools from connected
  servers.

### Layer 4 — Smoke
- [ ] A mock MCP stdio server is started, a tool is called, and the result
  appears in the conversation.

## Notes

**MCP crate choice:**
- Prefer a thin in-house JSON-RPC client over heavy MCP crates for now. The
  protocol surface we need is small (`initialize`, `tools/list`, `tools/call`).
  This can be revisited in `crate-replacement-audit` if a mature Rust MCP
  crate emerges.

**Files touched:**
- `crates/runie-core/src/mcp.rs` (new)
- `crates/runie-agent/src/tools.rs` (register MCP tools)
- `crates/runie-agent/src/turn.rs` (include MCP tools)
- `crates/runie-tui/src/components/extensions_modal/` (consume MCP events)

**Out of scope:**
- MCP resources/prompts (tools only for now).
- OAuth for MCP servers.
- Marketplace / discovery UI.
