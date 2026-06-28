# Centralize built-in tool names

**Status**: todo
**Milestone**: R4
**Category**: Tools
**Priority**: P2

**Depends on**: none
**Blocks**: cleanup-small-duplicates-and-dead-code

## Description

The same set of built-in tool names and dispatch matches is repeated across the agent stack. This makes adding, renaming, or removing a built-in tool error-prone because the list must be edited in many places.

Current duplication:

- `crates/runie-agent/src/tool/mod.rs:34–45` (`BUILTIN_TOOL_NAMES`)
- `crates/runie-agent/src/tool_runner.rs:46–68` (`dispatch_tool` + `is_known_tool`)
- `crates/runie-agent/src/headless/mod.rs:307–325` (`build_tool_registry`)
- `crates/runie-agent/src/turn/mod.rs:238–258` (`build_tool_registry` with read-only filtering)
- `crates/runie-agent/src/inspector.rs:82–99` (`dispatch_tool`)
- `crates/runie-agent/src/tests/tools.rs:14–35` (`dispatch_tool` test helper)
- `crates/runie-core/src/tool/shim/minimax.rs:22` (`KNOWN_TOOLS` used by parsers)

## Acceptance Criteria

- [ ] Define the canonical built-in tool name list in exactly one place.
- [ ] Update every location above to reference the canonical list instead of repeating the names.
- [ ] Keep behavior identical: read-only filtering in `turn/mod.rs` and test dispatch in `tests/tools.rs` still work.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `builtin_tools_registered_once` — verifies the list is defined once and every consumer resolves to the same set.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `mock_turn_still_dispatches_builtin_tools` — runs a provider-replay turn that exercises built-in tools and confirms dispatch still works after centralization.

## Files touched

- `crates/runie-agent/src/tool/mod.rs`
- `crates/runie-agent/src/tool_runner.rs`
- `crates/runie-agent/src/headless/mod.rs`
- `crates/runie-agent/src/turn/mod.rs`
- `crates/runie-agent/src/inspector.rs`
- `crates/runie-agent/src/tests/tools.rs`
- `crates/runie-core/src/tool/shim/minimax.rs`

## Notes

- This is an independent, high-Pareto task: small, safe, and removes a duplication hotspot that slows agentic feature work.
- Prefer placing the canonical list in `runie-protocol` or a shared module that both `runie-core` and `runie-agent` can import.
- Out of scope: changing tool schemas, MCP boundary, or skill-hook logic.
