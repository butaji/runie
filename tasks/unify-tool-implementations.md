# Unify Tool Implementations

**Status**: todo
**Milestone**: R3
**Category**: Tools
**Priority**: P0

**Depends on**: (none)
**Blocks**: (none)

## Description

Tools are implemented twice: `runie-core/src/tool/` defines an async `Tool` trait + registry, while `runie-agent/src/tools.rs` defines a sync `Tool` enum and re-implements read/write/edit/list/bash/grep/find. Every tool bug must be fixed twice.

## Acceptance Criteria

- [ ] A single canonical `Tool` trait/registry exists (likely in `runie-core`).
- [ ] `runie-agent` implements or wraps the canonical trait; the duplicate `Tool` enum is removed.
- [ ] All built-in tool logic lives in one place.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `tool_registry_unique` — only one registry assembles built-in tools.
- [ ] `agent_tool_uses_core_trait` — agent turn calls tools through the canonical trait.

### Layer 2 — Event Handling
- [ ] `tool_call_event_matches_output` — a tool call produces the same event regardless of caller.

## Files touched

- `crates/runie-core/src/tool/mod.rs`
- `crates/runie-core/src/tool/*.rs`
- `crates/runie-agent/src/tools.rs`
- `crates/runie-agent/src/tools/*.rs`
- `crates/runie-agent/src/turn.rs`

## Notes

May require converting agent tool calls from sync to async or providing a sync shim.
