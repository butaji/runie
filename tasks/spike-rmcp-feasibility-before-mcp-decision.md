# Spike `rmcp` feasibility before MCP decision

## Status

**todo** — rmcp does have client functionality; spike is viable

## Context

MCP server config exists but there is no runtime client. `rmcp` v1.8.0 has client functionality behind feature flags:

| Feature | Description |
|---------|-------------|
| `client` | Client functionality |
| `transport-child-process` | Client-side stdio transport (spawns child process) |
| `transport-streamable-http-client` | Streamable HTTP client (transport-agnostic) |
| `transport-streamable-http-client-reqwest` | Streamable HTTP client with reqwest backend |

Current workspace only enables `server` feature. Spike should enable client features and test connectivity.

## Goal

Time-boxed spike to evaluate whether `rmcp` client can replace the custom MCP scaffolding. If feasible, wire it up; if not, remove MCP config.

## Acceptance criteria

1. **Unit tests** — Spike demonstrates a working `rmcp` client call to a local MCP server.
2. **E2E tests** — Mock MCP server tool call works end-to-end.
3. **Live tmux tests** — Configure an MCP server and invoke its tool.

## Spike Steps

1. Add client features to `Cargo.toml`:
   ```toml
   rmcp = { version = "1.8", features = ["schemars", "server", "client", "transport-child-process"] }
   ```

2. Create spike test in `crates/runie-core/src/mcp/spike_client.rs`:
   - Connect to a local MCP server via stdio transport
   - List tools and call a tool
   - Verify response matches expected format

3. Document findings:
   - Does the client support our transport needs (stdio)?
   - Does it work with the MCP servers users typically configure?
   - What are the async patterns required?

## Decision Criteria

- **Migrate**: rmcp client works for stdio and HTTP transports
- **Hybridize**: Use rmcp for schema generation, custom code for transport
- **Remove**: rmcp client doesn't meet our needs; remove MCP config

## Tests

### Unit tests
- `rmcp_client_connects_to_stdio_server` — spike test with a simple MCP server.

### E2E tests
- N/A (spike).

### Live tmux tests
- N/A (spike).

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** N/A (spike).
- [ ] **Trigger events:** N/A (spike).
- [ ] **Observer events:** N/A (spike).
- [ ] **No direct mutations:** N/A (spike).
- [ ] **No new mirrors:** N/A (spike).
- [ ] **Async work observed:** N/A (spike).
