# Orchestrator Domain Types

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P0

**Depends on**: r4-adr-team-mode-orchestration
**Blocks**: r4-model-trait-resolution, r4-ask-user-tool, r4-one-shot-orchestrator-llm

## Description

Define the pure data structures that represent an Orchestrator plan:
`OrchestratorPlan`, `SubagentTask`, `TaskStatus`, and `PlanResult`. These types
must be serializable and usable by the runtime without any TUI dependencies.

## Acceptance Criteria

- [ ] `OrchestratorPlan` contains a list of `SubagentTask` items, a target model
  trait for synthesis, and an optional user-facing summary.
- [ ] `SubagentTask` has `id`, `role_prompt`, `task_description`, `tool_filter`,
  `model_trait`, `status`, and `output`.
- [ ] `TaskStatus` enum covers `Pending`, `Running`, `AwaitingUser`, `Done`,
  `Failed`.
- [ ] Types implement `Serialize`/`Deserialize` and a small property-based or
  round-trip JSON test.
- [ ] Linter guardrails in `crates/runie-core/build.rs` still pass.

## Tests

### Layer 1 — State / Logic

```rust
#[test]
fn plan_serializes_round_trip() {
    let plan = OrchestratorPlan {
        tasks: vec![SubagentTask {
            id: "t1".into(),
            role_prompt: "You are a code reviewer.".into(),
            task_description: "Review src/main.rs.".into(),
            tool_filter: None,
            model_trait: ModelTrait::Reasoning,
            status: TaskStatus::Pending,
            output: None,
        }],
        synthesis_trait: ModelTrait::General,
        summary: None,
    };
    let json = serde_json::to_string(&plan).unwrap();
    let decoded: OrchestratorPlan = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.tasks.len(), 1);
    assert_eq!(decoded.tasks[0].id, "t1");
}
```

```rust
#[test]
fn task_status_transitions_are_valid() {
    assert!(TaskStatus::Pending.can_transition_to(TaskStatus::Running));
    assert!(TaskStatus::Running.can_transition_to(TaskStatus::Done));
    assert!(!TaskStatus::Done.can_transition_to(TaskStatus::Pending));
}
```

## Files touched

- `crates/runie-core/src/orchestrator.rs` (new)
- `crates/runie-core/src/lib.rs`

## Out of scope

- LLM prompt construction.
- Actor runtime wiring.
- Subagent execution.
