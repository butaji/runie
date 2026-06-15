# Integrate `runie-core::tool::Tool` into Agent Turn

**Status**: done
**Milestone**: R3
**Category**: Tools
**Priority**: P0

**Depends on**: (none)
**Blocks**: mcp-client-integration, permission-rulesets

## Description

The `Tool` trait, `ToolRegistry`, and all built-in tool implementations already live in `crates/runie-core/src/tool/mod.rs`. This task integrated the core trait into the agent turn and fixed the emit callback signature.

## Acceptance Criteria

- [x] `runie-agent/src/turn.rs` calls tools via `ToolRegistry` and `serde_json::Value` arguments.
- [x] `runie-agent/src/parser.rs` produces `ParsedToolCall` (name + args JSON).
- [x] `runie-agent/src/headless.rs` uses `Arc<Mutex<dyn FnMut>>` for emit callback (Send compliance).
- [x] `runie-agent/src/subagent.rs` uses Arc<Mutex<...>> for state collection.
- [x] `cargo test --workspace` succeeds.

## Notes

- The emit callback now uses `Arc<Mutex<dyn FnMut(Event) + Send + Sync>>` pattern
- `emit_now()` helper function handles locking
- Both streaming deltas (ResponseDelta) and complete responses (Response) are emitted
- The unified registry is the extension point for future tools such as the FFF `search` and `find_definitions` tools (`docs/adr/0023-fff-search-integration.md`)
