# Unify Tool Implementations

**Status**: done
**Milestone**: R3
**Category**: Tools
**Priority**: P0

**Depends on**: (none)
**Blocks**: (none)

## Description

Tools are implemented twice: `runie-core/src/tool/` defines an async `Tool` trait + registry, while `runie-agent/src/tools.rs` defines a sync `Tool` enum and re-implements read/write/edit/list/bash/grep/find. Every tool bug must be fixed twice.

## Acceptance Criteria

- [x] A single canonical `Tool` trait/registry exists (likely in `runie-core`).
- [x] `runie-agent` implements or wraps the canonical trait; the duplicate `Tool` enum is removed.
- [x] All built-in tool logic lives in one place.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `tool_registry_unique` — only one registry assembles built-in tools.
- [x] `agent_tool_uses_core_trait` — agent turn calls tools through the canonical trait.

### Layer 2 — Event Handling
- [x] `tool_call_event_matches_output` — a tool call produces the same event regardless of caller.

## Files touched

- `crates/runie-core/src/tool/mod.rs`
- `crates/runie-core/src/tool/*.rs`
- `crates/runie-agent/src/tools.rs`
- `crates/runie-agent/src/tools/*.rs`
- `crates/runie-agent/src/turn.rs`

## Notes

`runie-agent` tests now call tools through `runie_core::tool::builtin_registry()`
and the canonical `Tool` trait. The duplicate `Tool` enum dispatch in
`runie-agent` was removed by the sub-agent refactor; core tools received
limit/offset fixes (`find` fallback now respects `limit`; `read_file` reports
`[Lines X-Y of Z]` when slicing). `agent_tool_uses_core_trait` and
`tool_call_event_matches_output` pass.
