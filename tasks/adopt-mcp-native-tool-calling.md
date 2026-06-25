# Adopt MCP and native tool calling

**Status**: in_progress
**Milestone**: R4
**Category**: Tools
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Summary

Replace the custom `Tool` trait, `ToolRegistry`, `define_tool!` macro, and text-based tool-call parsers with `rmcp` + provider-native function calling. Tool input schemas are derived from Rust structs via `schemars`.

## Implementation Progress

### Completed
- [x] `rmcp` (v1.8) added to workspace dependencies with `schemars` and `server` features
- [x] `schemars` (v1.0) added to workspace dependencies
- [x] New `tool/schema.rs` module with `ToolDef` trait and schema generation helpers
- [x] Typed input structs with `#[derive(JsonSchema)]` for `ReadFileInput` and `ListDirInput`
- [x] `generate_schema<T>()` function for schema generation from Rust types
- [x] `to_mcp_tool<T>()` and `to_openai_function<T>()` for provider integration
- [x] Updated `list_dir` and `read_file` tools to use typed schemas
- [x] `cargo check --workspace` is green

### Remaining
- [ ] Remove `crates/runie-core/src/tool_parser/*` (text-based parsers)
- [ ] Remove `crates/runie-agent/src/tool/define.rs` (macro)
- [ ] Migrate remaining tools to typed schema approach
- [ ] Update providers to use native function calling when available

## Acceptance Criteria

- [x] `rmcp` and `schemars` for tool schemas are added to workspace dependencies.
- [ ] `crates/runie-core/src/tool_parser/*`, `tool/define.rs`, and `tool/registry.rs` are removed.
- [x] Tools are defined via derive macros or `rmcp` `#[tool]` with typed input structs.
- [ ] Providers that support native function calling use it; a minimal fallback is kept only for text-only models.
- [x] The approval/permission gate is preserved around tool execution.
- [x] `cargo check --workspace` is green with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `schema_generation_produces_valid_schema` — verifies JSON schema output
- [x] `schema_includes_path_property` — verifies schema contains expected properties
- [x] `parse_input_round_trips` — verifies serialization/deserialization
- [x] `input_deserializes_required` — verifies required field parsing
- [x] `input_deserializes_full` — verifies optional field parsing
- [x] `slice_content_*` — verifies content slicing logic

### Layer 2 — Event Handling
- [ ] Tool dispatch and approval flow event handling tests

### Layer 3 — Rendering
N/A — no rendering changes

### Layer 4 — Smoke / Integration
- [ ] Provider-replay tests with tool-call streaming fixtures

## Files Changed

- `Cargo.toml` — added rmcp and schemars workspace dependencies
- `crates/runie-core/Cargo.toml` — added rmcp and schemars dependencies
- `crates/runie-agent/Cargo.toml` — added rmcp and schemars dependencies
- `crates/runie-core/src/tool/mod.rs` — exports new schema module
- `crates/runie-core/src/tool/schema.rs` — new schema-based tool definitions
- `crates/runie-agent/src/tool/list_dir.rs` — typed input with schemars
- `crates/runie-agent/src/tool/read_file.rs` — typed input with schemars
- `crates/runie-core/src/config.rs` — removed schema feature gates
- `crates/runie-core/src/model/state/types.rs` — removed schema feature gates
- `config.schema.json` — regenerated with schemars 1.0
