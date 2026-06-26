# Built-in subagent types as declarative data

**Status**: todo
**Milestone**: R4
**Category**: Agent
**Priority**: P1

**Depends on**: leader-actor-shared-runtime
**Blocks**: none

## Summary

Ship built-in subagent types (`explore`, `plan`, `verify`, `check-work`) as declarative data files (prompt template + input schema + model preferences) instead of ad-hoc code. The subagent runner loads and executes them generically.

## Acceptance Criteria

- Subagent types live as markdown/YAML files under `resources/agents/` and/or `~/.runie/agents/`.
- Each file declares `name`, `description`, `prompt`, `model` (optional), and `input_schema`.
- `SubagentRegistry` loads all subagent types at startup.
- Existing subagent runner dispatches through these definitions.
- Users can add custom subagent types by dropping a file; no Rust code changes required.
- `cargo check --workspace` is green.

## Tests

- **Layer 1**: Parse subagent type files and validate schemas.
- **Layer 4**: Provider-replay test that spawns a built-in subagent.
