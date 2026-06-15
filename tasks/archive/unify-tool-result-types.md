# Unify Tool Result Types

**Status**: done
**Milestone**: R3
**Category**: Core / State
**Priority**: P0

**Depends on**: unify-message-types
**Blocks**: (none)

## Description

Tool execution results are represented by three incompatible types:

- `runie-core::tool::ToolOutput` — `{ content, bytes_transferred, duration, status }`.
- `runie-agent::tools::ToolResult` — `{ tool, output, success }`.
- `runie-agent::headless::ToolOutput` — `{ name, arguments, output }`.

This forces the tool pipeline to convert between shapes and obscures the
single data contract: a tool ran, it succeeded or failed, and it produced
output.

## Acceptance Criteria

- [ ] A single `ToolResult`/`ToolOutput` type lives in `runie-core`.
- [ ] `runie-agent::tools` uses the canonical type and deletes its local
  `ToolResult`.
- [ ] `runie-agent::headless` uses the canonical type and deletes its local
  `ToolOutput`.
- [ ] The canonical type captures success/error status, rendered output,
  and optional structured shell metadata without duplication.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `tool_result_success_and_failure` — canonical type distinguishes
  success, error, and blocked states.
- [ ] `bash_tool_produces_canonical_output` — `bash` execution returns the
  canonical type.
- [ ] `headless_turn_uses_canonical_tool_output` — `run_headless_turn`
  populates the canonical type.

### Layer 2 — Event Handling
- [ ] `agent_tool_end_event_carries_canonical_result` —
  `AgentEvent::ToolEnd` uses the canonical type.

## Files touched

- `crates/runie-core/src/tool.rs`
- `crates/runie-agent/src/tools.rs`
- `crates/runie-agent/src/headless.rs`
- `crates/runie-agent/src/lib.rs`
- Call sites in `runie-agent` and `runie-core`.

## Notes

Keep rendered output and structured metadata in one type so that TUI,
session persistence, and headless modes all see the same data.
