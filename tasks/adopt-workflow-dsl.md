# Adopt Workflow DSL for Multi-Agent Orchestration

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P3

**Depends on**: `adopt-multi-agent-spawning`
**Blocks**: (none)

## Description

Create a declarative workflow DSL for defining Team mode orchestration:

### Syntax
```rust
// Start team mode for a task
/workflow "Research Rust async patterns" as researcher, "Write tests" as tester

// With explicit synthesis
/workflow "Research" as researcher
    --synthesize "Combine findings into a report"

// Parallel execution
/workflow [
    "Research web" as web-researcher,
    "Research docs" as doc-researcher,
]
```

### Built-in Synthesis Options
- **LLM synthesis** (default): Orchestrator uses LLM to combine results
- **Template**: `/workflow <tasks> --template "Results:\n{tasks}"`
- **Custom prompt**: `/workflow <tasks> --synthesize "Combine: {results}"`

### Key Design Decisions
- Depth = 1 (orchestrator + subagents only)
- Subagent naming: `{Role}-{3 alphanumeric}` (e.g., `researcher-A1B`)
- Retry: 3 retries → same-trait fallback → user escalation
- Steering: `/steer <agent> <message>`
- Cancellation: `/cancel <agent>`

Reference: `~/Code/agents/omegacode/` workflow DSL, `~/Code/agents/crewai/` crews/flows

## Acceptance Criteria

- [ ] `/workflow` command with task list and optional `as <name>` aliases.
- [ ] Parallel execution syntax: `[/workflow [...], [...]]`
- [ ] Synthesis options: `--synthesize`, `--template`
- [ ] Team mode activation via `/team` or `/workflow`
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `workflow_command_parses` — valid syntax parses correctly.
- [ ] `parallel_workflow_creates_multiple_agents` — list syntax works.
- [ ] `synthesis_options_accepted` — both --synthesize and --template work.

### Layer 2 — Event Handling
- [ ] `workflow_command_starts_orchestrator` — triggers team mode.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
- [ ] Smoke test: `/workflow "echo test" as tester` executes.

## Files touched

- `crates/runie-core/src/commands/workflow.rs` (new)
- `crates/runie-core/src/dsl/` (new module)

## Notes

High-complexity task. Defer until core multi-agent spawning is stable.
