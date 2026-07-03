# Spike `rmcp` feasibility before MCP decision

## Status

**done** ✅ — rmcp client works for stdio transport.

## Context

MCP server config exists but there was no runtime client. `rmcp` v1.8.0 has client functionality behind feature flags. This spike evaluated whether rmcp client can connect to an MCP server via stdio.

## Findings (2026-07-02)

### Decision: Migrate — rmcp client is viable

### API Surface

| Component | API | Notes |
|-----------|-----|-------|
| Transport | `TokioChildProcess::new(command.configure(\|c\| {...}))` | Spawns subprocess with piped stdin/stdout |
| Client | `serve_client((), transport).await?` | Connects and performs MCP handshake |
| Tool call | `client.list_all_tools().await?` | Returns `Vec<Tool>` |
| Shutdown | `client.cancel().await?` | Graceful disconnection |

### Feature Notes

- The `transport-child-process` feature enables `TokioChildProcess` which wraps `process-wrap`.
- The `local` feature (enabled via `rmcp-macros`) changes `serve_client` from async to sync.
  - In this workspace, `local` is NOT enabled, so `serve_client()` returns `impl Future` → needs `.await`.
- The spike test creates a Python echo server that speaks MCP JSON-RPC over stdin/stdout.

### Implementation Notes

1. `TokioChildProcess::new()` accepts `impl Into<CommandWrap>`. Use `.configure()` to set args and stdio:
   ```rust
   TokioChildProcess::new(
       tokio::process::Command::new("python3")
           .configure(|c| {
               c.arg("/path/to/server.py");
               c.stdout(Stdio::piped());
               c.stdin(Stdio::piped());
               c.stderr(Stdio::piped());
           }),
   )?
   ```

2. `()` implements `ClientHandler` and `Service<RoleClient>` via blanket impls.
   Use `rmcp::serve_client((), transport)` to connect.

3. After handshake, the `RunningService<RoleClient, ()>` provides:
   - `list_all_tools()` → `Vec<Tool>`
   - `list_all_resources()` → `Vec<Resource>`
   - `call_tool(name, args)` → `CallToolResult`
   - `cancel()` → graceful shutdown

### Next Steps (from spike)

1. **Replace placeholder code** in `crates/runie-core/src/mcp/connection.rs`:
   - Use `TokioChildProcess::new(command.configure(...))` instead of the TODO stub
   - Use `serve_client((), transport)` to connect
   - Use `list_all_tools()` to get tool schemas
   - Cache schemas in the existing `SchemaCache`

2. **Update `McpConnectionManager`**:
   - Spawn one `TokioChildProcess` per server config
   - The existing `cache_dir` and `SchemaCache` infrastructure works as-is

3. **Test with real MCP servers**:
   - `npx @modelcontextprotocol/server-filesystem /tmp` — filesystem tools
   - Custom MCP servers via stdio or HTTP

## Acceptance criteria

- [x] **Unit tests** — Spike test `rmcp_client_connects_to_echo_server` demonstrates a working rmcp client call to a local Python MCP server.
- [x] **E2E tests** — N/A (spike).
- [x] **Live tmux tests** — N/A (spike).

## Tests

### Layer 1 — State/Logic
- `rmcp_client_connects_to_echo_server` — spawns a Python echo MCP server, connects, lists tools, and verifies the response.

## Files touched

- `Cargo.toml` (workspace) — added `transport-child-process` feature to rmcp
- `crates/runie-core/src/mcp/spike_client.rs` — spike test
- `crates/runie-core/src/mcp/mod.rs` — added `#[cfg(test)] mod spike_client`
