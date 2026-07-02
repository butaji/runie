# Wire `rmcp` client or remove MCP config

## Status

**partial** — rmcp client viability confirmed (spike done); actual wiring not yet implemented.

## Description

MCP server config exists but there was no runtime client. The `spike-rmcp-feasibility-before-mcp-decision.md` confirmed that `rmcp` client works for stdio transport.

**Decision: Migrate** — wire `rmcp` client into `McpConnectionManager`.

## Changes from spike

### Phase 1: Viability confirmed ✅
- `rmcp::serve_client((), transport).await?` connects to a subprocess via stdio
- `TokioChildProcess::new(command.configure(...))` spawns the MCP server process
- `client.list_all_tools().await?` returns tool schemas
- `client.cancel().await?` graceful shutdown

### Phase 2: Wiring (not yet done)
Remaining work:
1. Replace the TODO placeholder in `crates/runie-core/src/mcp/connection.rs::start_server`
2. Use `TokioChildProcess::new(command.configure(...))` for stdio transport
3. Use `serve_client((), transport)` to perform MCP handshake
4. Use `list_all_tools()` to populate the tool schema cache
5. Update `McpConnectionManager` to manage the `RunningService<RoleClient, ()>`

## Acceptance criteria

- [ ] Unit tests — `rmcp_client_connects_to_stdio_server` passes; `to_mcp_tool` schema round-trip works.
- [ ] E2E tests — Mock MCP server tool call works end-to-end.
- [ ] Live tmux tests — Configure an MCP server in tmux and invoke its tool.

## Tests

### Layer 1 — State/Logic
- [x] `spike_rmcp_client_connects_to_echo_server` — Python echo MCP server, connect, list tools.

### Layer 4 — E2E
- [ ] Mock MCP server tool call via replay.

### Live tmux tests
- [ ] Add a simple MCP server and run a tool through it.

## Files touched

- `Cargo.toml` (workspace) — added `transport-child-process` feature
- `crates/runie-core/src/mcp/spike_client.rs` — spike test
- `crates/runie-core/src/mcp/connection.rs` — wiring (not done yet)
- `crates/runie-core/src/mcp/mod.rs` — spike module added
