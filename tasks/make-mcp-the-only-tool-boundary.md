# Make MCP the only tool boundary

**Status**: done
**Milestone**: R4
**Category**: Tools
**Priority**: P0

**Depends on**: adopt-mcp-native-tool-calling
**Blocks**: none

## Summary

Move from a hybrid tool stack to a single boundary: all tools are MCP tools via `ToolDef` trait. Delete the legacy `Tool` trait, `ToolRegistry`, `define_tool!` macro, and text/markup/inline-JSON tool parsers. Tool approval and execution use static dispatch.

## Changes Made

### runie-core
- `tool/mod.rs`: Export parse module and `ParsedToolCall`/`ToolParseError` types
- `tool/tests.rs`: Updated to use `ToolDef` trait instead of legacy `Tool` trait

### runie-agent
- `tool/mod.rs`: Re-export `ToolDef` from `runie_core::tool`
- `tool_runner.rs`: Replace dynamic dispatch with static dispatch via `dispatch_tool()` match
- `inspector.rs`: Replace `ToolRegistry` with static dispatch pattern
- `grep_find.rs`: Remove `builtin_registry()` usage
- `tests/tools.rs`: Update to use static dispatch pattern
- `tool/find_definitions.rs`: Fix return type handling
- `tool/search/core.rs`: Refactor to extract error handling and fix type mismatches
- `tool/search/tests.rs`: Update to use `ToolDef` trait

## Acceptance Criteria

- ✅ Every built-in tool is exposed as an MCP tool with a `schemars`-derived input schema.
- ✅ Legacy files `tool/define.rs` and `tool/registry.rs` removed (already done).
- ✅ `runie-agent` uses static dispatch instead of dynamic dispatch via `ToolRegistry`.
- ✅ Permission policy is handled by `PermissionGate`.
- ✅ Text-only providers use the parse module as a fallback shim.
- ✅ `cargo check --workspace` is green.

## Tests

- **Layer 1**: Schema round-trip and argument validation tests. ✅
- **Layer 2**: Permission interceptor event handling. ✅
- **Layer 4**: Provider-replay tests with tool-call streaming fixtures. ✅

## Files Changed

- `crates/runie-core/src/tool/mod.rs`
- `crates/runie-core/src/tool/tests.rs`
- `crates/runie-agent/src/tool/mod.rs`
- `crates/runie-agent/src/tool_runner.rs`
- `crates/runie-agent/src/inspector.rs`
- `crates/runie-agent/src/grep_find.rs`
- `crates/runie-agent/src/tests/tools.rs`
- `crates/runie-agent/src/turn/mod.rs`
- `crates/runie-agent/src/headless/mod.rs`
- `crates/runie-agent/src/stream_response.rs`
- `crates/runie-agent/src/tool/find_definitions.rs`
- `crates/runie-agent/src/tool/search/core.rs`
- `crates/runie-agent/src/tool/search/tests.rs`
