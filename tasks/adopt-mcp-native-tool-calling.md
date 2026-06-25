# Adopt MCP and native tool calling

**Status**: todo
**Milestone**: R4
**Category**: Tools
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Summary

Replace the custom `Tool` trait, `ToolRegistry`, `define_tool!` macro, and text-based tool-call parsers with `rmcp` + provider-native function calling. Tool input schemas are derived from Rust structs via `schemars`.

## Acceptance Criteria

- `rmcp` and `schemars` for tool schemas are added to workspace dependencies.
- `crates/runie-core/src/tool_parser/*`, `tool/define.rs`, and `tool/registry.rs` are removed.
- Tools are defined via derive macros or `rmcp` `#[tool]` with typed input structs.
- Providers that support native function calling use it; a minimal fallback is kept only for text-only models.
- The approval/permission gate is preserved around tool execution.
- `cargo check --workspace` is green with no new warnings.

## Tests

- **Layer 1**: Schema generation and argument deserialization/validation tests.
- **Layer 2**: Tool dispatch and approval flow event handling.
- **Layer 4**: Provider-replay tests with tool-call streaming fixtures.
