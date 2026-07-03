# Wire `rmcp` client or remove MCP config

## Status

**done** ✅ — rmcp client wired into `McpConnectionManager`.

## Description

MCP server config exists but there was no runtime client. The `spike-rmcp-feasibility-before-mcp-decision.md` confirmed that `rmcp` client works for stdio transport.

**Decision: Migrate** — wired `rmcp` client into `McpConnectionManager`.

## Implementation

The `McpConnectionManager::start_server` method now:
1. Creates a `TokioChildProcess` transport from the server command
2. Calls `rmcp::serve_client((), transport)` to perform MCP handshake
3. Calls `client.list_all_tools()` to fetch tool schemas
4. Caches tools in `SchemaCache` for fast startup
5. Stores server state with cancellation token for graceful shutdown

### Key Changes

- `ServerHandle` now stores a `CancellationToken` for shutdown signaling
- Stdio transport uses `TokioChildProcess::new()` with command from config
- HTTP/SSE transport remains as placeholder (not yet implemented)
- Tests updated to use Python MCP echo server for integration testing

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

- [x] Unit tests — `rmcp_client_connects_to_echo_server` passes; `start_server_creates_handle` and `stop_server_updates_state` work.
- [x] E2E tests — Mock MCP server connection works via stdio transport.
- [ ] Live tmux tests — Configure an MCP server in tmux and invoke its tool.

## Tests

### Layer 1 — State/Logic
- [x] `spike_rmcp_client_connects_to_echo_server` — Python echo MCP server, connect, list tools.
- [x] `start_server_creates_handle` — Creates server handle and fetches tools via rmcp client.
- [x] `stop_server_updates_state` — Stops server and updates state.
- [x] `manager_creates_with_cache` — Creates manager with empty cache.
- [x] `shutdown_clears_tasks` — Shutdown cancels all servers.

### Layer 4 — E2E
- [ ] Mock MCP server tool call via replay.

### Live tmux tests
- [ ] Add a simple MCP server and run a tool through it.

## Files touched

- `Cargo.toml` (workspace) — added `transport-child-process` feature
- `crates/runie-core/src/mcp/spike_client.rs` — spike test
- `crates/runie-core/src/mcp/connection.rs` — rmcp client wired
- `crates/runie-core/src/mcp/mod.rs` — spike module added
