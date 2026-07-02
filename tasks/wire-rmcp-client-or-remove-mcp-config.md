# Wire `rmcp` client or remove MCP config

## Status

`todo`

## Description

MCP server config exists but there is no runtime client. Either wire `rmcp` client/runtime to invoke external MCP servers, or remove the dead config.

## Acceptance criteria

1. **Unit tests** — `rmcp` client connects to a local MCP server and invokes a tool, or MCP config is removed.
2. **E2E tests** — A mock MCP server tool call works end-to-end, or no MCP code remains.
3. **Live tmux tests** — Configure an MCP server in tmux and invoke its tool, or confirm MCP settings are gone.

## Tests

### Unit tests
- `to_mcp_tool` schema round-trip; client initialization if implemented.

### E2E tests
- Mock MCP server tool call via replay.

### Live tmux tests
- Add a simple MCP server and run a tool through it.
