# Add PlanActor for plan-first execution

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: turn-actor-owns-agent-turn-state
**Blocks**: none

## Summary

Introduce a `PlanActor` that owns the plan graph. The agent proposes a plan; the user approves it; `TurnActor` executes steps only after `PlanApproved`. This replaces imperative turn logic with a declarative plan state machine.

## Acceptance Criteria

- `PlanActor` owns a graph of plan nodes (steps, dependencies, status).
- New intents: `CreatePlan`, `ApprovePlan`, `RejectPlan`, `UpdatePlanStep`.
- New facts: `PlanCreated`, `PlanChanged`, `PlanApproved`, `PlanStepCompleted`.
- Write tools are blocked until `PlanApproved` is emitted.
- The TUI renders the plan graph from facts.
- `cargo check --workspace` is green.

## Tests

- **Layer 1**: Plan graph state transitions and dependency resolution.
- **Layer 2**: Plan approval flow event handling.
- **Layer 4**: Provider-replay test where the model proposes a multi-step plan.
