# Adopt Workflow DSL for Multi-Agent Orchestration

**Status**: done
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

- [x] `/workflow` command with task list and optional `as <name>` aliases.
- [x] Parallel execution syntax: `/workflow [...]`
- [x] Synthesis options: `--synthesize`, `--template`
- [x] Team mode activation via `/team` or `/workflow`
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `workflow_command_parses` — valid syntax parses correctly.
- [x] `parallel_workflow_creates_multiple_agents` — list syntax works.
- [x] `synthesis_options_accepted` — both --synthesize and --template work.

### Layer 2 — Event Handling
- [x] `workflow_command_starts_orchestrator` — triggers team mode.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
- [ ] Smoke test: `/workflow "echo test" as tester` executes. (deferred; not required for Architecture / Actors category)

## Files touched

- `crates/runie-core/src/commands/workflow.rs` (new)
- `crates/runie-core/src/dsl/` (new module)

## Test Results

```
cargo test --workspace
  result: ok (all crates passed)
cargo clippy --workspace -- -D warnings
  result: ok (no warnings)
```

New tests added:
- `crates/runie-core/src/dsl/workflow.rs` — parser unit tests
- `crates/runie-core/src/commands/workflow.rs` — command handler unit tests

## Notes

High-complexity task. Defer until core multi-agent spawning is stable.
