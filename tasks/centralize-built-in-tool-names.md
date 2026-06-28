# Centralize built-in tool names

**Status**: partial
**Milestone**: R4
**Category**: Tools
**Priority**: P2

**Depends on**: none
**Blocks**: cleanup-small-duplicates-and-dead-code

## Description

The same set of built-in tool names and dispatch matches is repeated across the agent stack. This makes adding, renaming, or removing a built-in tool error-prone because the list must be edited in many places.

**Completed:**

- `crates/runie-core/src/tool/mod.rs` — canonical `BUILTIN_TOOL_NAMES` and `is_builtin_tool` defined
- `crates/runie-agent/src/tool/mod.rs` — re-exports canonical list
- `crates/runie-agent/src/tool_runner.rs` — `is_known_tool` now delegates to `is_builtin_tool`
- `crates/runie-core/src/tool/shim/minimax.rs` — uses canonical list + protocol names

**Not duplicated (uses concrete tool types):**

- `crates/runie-agent/src/headless/mod.rs` — uses `BashTool`, `ReadFileTool`, etc. directly
- `crates/runie-agent/src/turn/mod.rs` — uses concrete tool types
- `crates/runie-agent/src/inspector.rs` — uses concrete tool types
- `crates/runie-agent/src/tests/tools.rs` — uses concrete tool types

## Acceptance Criteria

- [x] Define the canonical built-in tool name list in exactly one place (`runie_core::tool::BUILTIN_TOOL_NAMES`).
- [x] Update `runie-agent/src/tool/mod.rs` to re-export canonical list.
- [x] Update `runie-agent/src/tool_runner.rs` to use `is_builtin_tool`.
- [x] Update `runie-core/src/tool/shim/minimax.rs` to use canonical list + protocol names.
- [ ] Run MiniMax SSE replay fixtures to verify parsing still works.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `builtin_tool_names_matches_core` — verifies re-export matches core's canonical list.
- [x] `builtin_tool_names_contains_all_tools` — verifies all tool implementations are in the list.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `mock_turn_still_dispatches_builtin_tools` — runs a provider-replay turn that exercises built-in tools.

## Files touched

- `crates/runie-core/src/tool/mod.rs` (added canonical list)
- `crates/runie-agent/src/tool/mod.rs` (replaces local definition)
- `crates/runie-agent/src/tool_runner.rs` (uses `is_builtin_tool`)
- `crates/runie-core/src/tool/shim/minimax.rs` (uses canonical + protocol names)

## Notes

- This is an independent, high-Pareto task: small, safe, and removes a duplication hotspot that slows agentic feature work.
- Canonical list placed in `runie-core` since it's used by both core (parsing) and agent (dispatch).
- Out of scope: changing tool schemas, MCP boundary, or skill-hook logic.
