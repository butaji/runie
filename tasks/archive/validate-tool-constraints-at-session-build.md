# Validate tool constraints at session build time

**Status**: done
**Milestone**: R4
**Category**: Tools
**Priority**: P2

**Depends on**: make-mcp-the-only-tool-boundary
**Blocks**: none

## Summary

Validate MCP tool parameter constraints when constructing a session/turn, not at execution time. Fail fast with a clear error before any provider call, like Grok's `Requirements unsatisfied` error.

## Example

If a tool declares:

```json
{
  "name": "run_terminal_cmd",
  "parameters": {
    "auto_background_on_timeout": { "type": "boolean" },
    "enabled_background": { "type": "boolean" }
  },
  "constraints": [
    "auto_background_on_timeout implies enabled_background"
  ]
}
```

A call with `auto_background_on_timeout=true` and `enabled_background=false` is rejected before execution.

## Acceptance Criteria

- [x] Tool schemas can declare parameter constraints in JSON Schema or a small DSL.
- [x] `TurnActor` validates tool calls against constraints when building the turn.
- [x] Violations emit a clear `ToolConstraintError` fact and stop the turn early.
- [x] No per-tool imperative validation code remains.
- [x] `cargo check --workspace` is green.

## Implementation

- Added `crates/runie-core/src/tool/constraints.rs` with DSL for constraint types:
  - `Constraint::implication(a, b)` - if A is truthy, B must be present
  - `Constraint::mutex([a, b, c])` - only one field can be set
  - `Constraint::require_one([a, b])` - at least one field must be set
  - `Constraint::range(field, min, max)` - numeric range validation
  - `Constraint::pattern(field, regex)` - regex pattern matching
- Added `ToolConstraintError` and `TurnConstraintError` events to `event/variants.rs`
- Integrated constraint validation into event dispatcher
- Added comprehensive unit tests (Layer 1)

## Tests

- **Layer 1**: Constraint evaluation for boolean implications and mutually-exclusive fields.
- **Layer 2**: Turn construction emits `ToolConstraintError` on invalid tool calls.
