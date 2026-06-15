# One-Shot Orchestrator LLM Planner

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P0

**Depends on**: llm-event-normalization, r4-model-trait-resolution, r4-ask-user-tool
**Blocks**: r4-orchestrator-actor

## Description

Implement the Orchestrator's planning phase: given a user request, project
context, and any Ask-User answers, call the planner model once and parse a
structured `OrchestratorPlan` from JSON. The LLM must output valid OHP JSON that
conforms to a strict schema.

## Acceptance Criteria

- [ ] Planner prompt includes system instructions, available traits, available
  tools, project context, and the user request.
- [ ] LLM output is parsed into `OrchestratorPlan` with robust error handling.
- [ ] Failed parse triggers a retry with a clearer prompt up to `N` times
  (configurable, default 2).
- [ ] Plan validation ensures each task references only available tools and a
  resolvable model trait.
- [ ] Planner is deterministic enough to be tested with a mock provider.

## Tests

### Layer 1 — State / Logic

```rust
#[test]
fn planner_parses_valid_ohip_json() {
    let mock = MockProvider::with_response(include_str!("fixtures/plan.json"));
    let planner = OneShotPlanner::new(mock, resolver());
    let plan = planner.plan("Review the codebase", &Context::default()).unwrap();
    assert_eq!(plan.tasks.len(), 2);
}

#[test]
fn planner_retries_on_invalid_json() {
    let mock = MockProvider::sequence(&["not json", include_str!("fixtures/plan.json")]);
    let planner = OneShotPlanner::new(mock, resolver()).with_max_retries(2);
    let plan = planner.plan("Fix bug", &Context::default()).unwrap();
    assert_eq!(plan.tasks.len(), 1);
}
```

### Layer 2 — Event Handling

```rust
#[test]
fn plan_validation_rejects_unknown_trait() {
    let mut plan = sample_plan();
    plan.tasks[0].model_trait = ModelTrait::Custom("unknown".into());
    assert!(validate_plan(&plan, &resolver()).is_err());
}
```

## Files touched

- `crates/runie-provider/src/planner.rs` (new)
- `crates/runie-provider/src/lib.rs`
- `crates/runie-core/src/orchestrator.rs`

## Out of scope

- Multi-shot refinement loop.
- Recursive planning (future).
