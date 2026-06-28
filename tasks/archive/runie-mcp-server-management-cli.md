# `runie mcp` server management CLI

**Status**: done
**Milestone**: R4
**Category**: Tools
**Priority**: P1

**Depends on**: make-mcp-the-only-tool-boundary
**Blocks**: none

## Summary

Add `runie mcp add/list/remove` to manage MCP server configurations. Supports stdio, HTTP, and SSE transports in user (`~/.runie/config.toml`) or project (`./.runie/config.toml`) scope.

## Example

```bash
# User-scoped stdio server
runie mcp add filesystem npx -y @modelcontextprotocol/server-filesystem ~/Code \
  --transport stdio --scope user

# Project-scoped remote server
runie mcp add fetch https://api.example.com/mcp \
  --transport sse --scope project \
  -H "Authorization: Bearer ${TOKEN}"

runie mcp list
runie mcp remove filesystem --scope user
```

Persisted config:

```toml
[mcp.servers.filesystem]
transport = "stdio"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/Users/admin/Code"]
scope = "user"

[mcp.servers.fetch]
transport = "sse"
url = "https://api.example.com/mcp"
headers = { Authorization = "Bearer ${TOKEN}" }
scope = "project"
```

## Acceptance Criteria

- `runie mcp list` shows configured servers with transport and scope.
- `runie mcp add <name> <command-or-url> --transport stdio|http|sse --scope user|project` persists config.
- `runie mcp remove <name> --scope user|project` deletes config.
- Environment variables and headers can be attached.
- The leader/runtime picks up MCP servers from config without code changes.
- `cargo check --workspace` is green.

## Tests

- **Layer 1**: Config serialization/deserialization for MCP entries.
- **Layer 2**: CLI command events for add/list/remove.
