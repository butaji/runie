# Validate tool constraints at session build time

**Status**: todo
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

- Tool schemas can declare parameter constraints in JSON Schema or a small DSL.
- `TurnActor` validates tool calls against constraints when building the turn.
- Violations emit a clear `ToolConstraintError` fact and stop the turn early.
- No per-tool imperative validation code remains.
- `cargo check --workspace` is green.

## Tests

- **Layer 1**: Constraint evaluation for boolean implications and mutually-exclusive fields.
- **Layer 2**: Turn construction emits `ToolConstraintError` on invalid tool calls.
