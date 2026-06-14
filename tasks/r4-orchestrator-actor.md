# Orchestrator Actor

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: r3-actor-runtime-decision, r4-orchestrator-domain-types, r4-one-shot-orchestrator-llm
**Blocks**: r4-subagent-isolation, r4-subagent-sidebar, r4-team-mode-integration

## Description

Create an `OrchestratorActor` that lives in the actor runtime, owns the
Orchestrator state machine, and coordinates subagent execution. It receives
user requests, runs the one-shot planner, dispatches subagent tasks, collects
results, runs synthesis, and emits feed events.

## Acceptance Criteria

- [ ] `OrchestratorActor` implements the actor `Handle` trait.
- [ ] States: `Idle`, `Aligning`, `Planning`, `Executing`, `Synthesizing`,
  `Done`, `Failed`.
- [ ] Emits events for state transitions, subagent status changes, and final
  synthesis.
- [ ] Persists its state to the event bus so recovery works after restart.
- [ ] Handles cancellation gracefully when the user switches back to Solo mode
  or aborts.

## Tests

### Layer 1 — State / Logic

```rust
#[test]
fn orchestrator_transitions_idle_to_aligning() {
    let mut orch = OrchestratorActor::default();
    orch.start_request("Refactor auth".into(), Context::default());
    assert!(matches!(orch.state, OrchestratorState::Aligning(_)));
}

#[test]
fn orchestrator_collects_subagent_results() {
    let mut orch = OrchestratorActor::default();
    orch.apply_event(OrchestratorEvent::SubagentDone { id: "t1".into(), result: json!("ok") });
    assert_eq!(orch.results.len(), 1);
}
```

### Layer 2 — Event Handling

```rust
#[test]
fn cancellation_returns_to_idle() {
    let mut orch = OrchestratorActor::default();
    orch.start_request("task".into(), Context::default());
    orch.handle(OrchestratorCommand::Cancel);
    assert!(matches!(orch.state, OrchestratorState::Idle));
}
```

## Files touched

- `crates/runie-core/src/actors/orchestrator.rs` (new)
- `crates/runie-core/src/actors/mod.rs`
- `crates/runie-core/src/lib.rs`

## Out of scope

- Subagent actor implementation.
- Sidebar UI.
