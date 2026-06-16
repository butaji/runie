# Adopt Workflow DSL for Multi-Agent Orchestration

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P3

**Depends on**: `adopt-multi-agent-spawning`
**Blocks**: (none)

## Description

Create a declarative workflow DSL for defining complex multi-agent orchestration:

```rust
// Define a research workflow
workflow!(ResearchWorkflow, |input| {
    // Start with a coordinator agent
    let coordinator = agent!("coordinator")
        .model("claude-sonnet-4")
        .system_prompt("You coordinate research teams.");

    // Parallel research agents
    let research = parallel!([
        agent!("web-researcher").tool("search"),
        agent!("code-researcher").tool("grep").tool("read"),
        agent!("doc-researcher").tool("read"),
    ]);

    // Pipeline: coordinator → parallel → aggregator
    pipeline![
        coordinator.prompt(input),
        research.spawn_for_each(|r| r.results),
        agent!("aggregator")
            .prompt("Synthesize: {:?}", research.results),
    ]
});

// Execute workflow
let results = workflow.run("Research Rust async patterns").await?;
```

Reference: `~/Code/agents/omegacode/` workflow DSL, `~/Code/agents/crewai/` crews/flows

## Acceptance Criteria

- [ ] `workflow!` macro for workflow definition.
- [ ] `agent!` macro for agent instantiation.
- [ ] `parallel!` for parallel execution.
- [ ] `pipeline!` for sequential composition.
- [ ] `spawn_for_each` for fan-out/fan-in patterns.
- [ ] Workflow execution with progress tracking.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `workflow_defines_correctly` — macro generates valid workflow.
- [ ] `parallel_executes_all` — parallel block runs all agents.
- [ ] `pipeline_chains_results` — pipeline passes results correctly.
- [ ] `spawn_for_each_fans_out` — fan-out creates correct number of agents.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
- [ ] `workflow_progress_renders` — progress display shows workflow state.

### Layer 4 — Smoke / Crash
- [ ] Smoke test: simple workflow executes.

## Files touched

- `crates/runie-workflow/` (new crate)
  - `src/dsl.rs`
  - `src/executor.rs`
  - `src/lib.rs`
- `crates/runie-macros/src/workflow.rs` (add to existing)

## Notes

High-complexity task. Defer until core multi-agent is stable.
