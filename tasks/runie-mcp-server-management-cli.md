# `runie mcp` server management CLI

**Status**: todo
**Milestone**: R4
**Category**: Tools
**Priority**: P1

**Depends on**: make-mcp-the-only-tool-boundary
**Blocks**: none

## Summary

Add `runie mcp add/list/remove` to manage MCP server configurations. Supports stdio, HTTP, and SSE transports in user (`~/.runie/config.toml`) or project (`./.runie/config.toml`) scope.

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
