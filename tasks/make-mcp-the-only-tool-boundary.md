# Make MCP the only tool boundary

**Status**: todo
**Milestone**: R4
**Category**: Tools
**Priority**: P0

**Depends on**: adopt-mcp-native-tool-calling
**Blocks**: none

## Summary

Move from a hybrid tool stack to a single boundary: all tools are MCP tools. Delete the legacy `Tool` trait, `ToolRegistry`, `define_tool!` macro, and text/markup/inline-JSON tool parsers. Tool approval and execution become MCP middleware.

## Acceptance Criteria

- Every built-in tool is exposed as an MCP tool with a `schemars`-derived input schema.
- `crates/runie-core/src/tool_parser/*`, `tool/define.rs`, and `tool/registry.rs` are removed.
- `runie-agent` no longer contains custom tool dispatch; it calls the MCP server/runtime.
- Permission policy is an MCP interceptor that returns `Allow`, `Ask`, or `Deny` before execution.
- Text-only providers use a minimal shim that converts native tool calls to/from text, not a permanent parser stack.
- `cargo check --workspace` is green.

## Tests

- **Layer 1**: Schema round-trip and argument validation tests.
- **Layer 2**: Permission interceptor event handling.
- **Layer 4**: Provider-replay tests with tool-call streaming fixtures.
