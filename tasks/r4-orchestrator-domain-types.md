# Orchestrator Domain Types

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P0

**Depends on**: r4-adr-team-mode-orchestration
**Blocks**: r4-model-trait-resolution, r4-ask-user-tool, r4-one-shot-orchestrator-llm

## Description

Define the pure data structures that represent an Orchestrator plan:
`OrchestratorPlan`, `SubagentTask`, `TaskStatus`, and `PlanResult`. These types
are serializable and usable by the runtime without any TUI dependencies.

## What was implemented

- `ModelTrait` enum: `Fast`, `General`, `Reasoning`, `Vision`, `LongContext`
  - With `label()`, `short_label()`, `Display` impl
- `TaskStatus` enum: `Pending`, `Running`, `AwaitingUser`, `Done`, `Failed`
  - `can_transition_to()` for lifecycle validation
  - `is_terminal()`, `label()`
- `SubagentTask`: `id`, `role_prompt`, `task_description`, `tool_filter`,
  `model_trait`, `status`, `output`
  - `SubagentTask::new()` builder
  - `is_runnable()`
- `OrchestratorPlan`: `tasks`, `synthesis_trait`, `summary`, `rationale`
  - `OrchestratorPlan::simple()` helper
  - `task_count()`, `completed_count()`, `is_complete()`, `is_unstarted()`
- `PlanResult`: `success`, `response`, `failures`, `elapsed_secs`
- `TaskFailure`: `task_id`, `error`

## Acceptance Criteria

- [x] `OrchestratorPlan` contains a list of `SubagentTask` items, a target model
  trait for synthesis, and an optional user-facing summary.
- [x] `SubagentTask` has `id`, `role_prompt`, `task_description`, `tool_filter`,
  `model_trait`, `status`, and `output`.
- [x] `TaskStatus` enum covers `Pending`, `Running`, `AwaitingUser`, `Done`,
  `Failed`.
- [x] Types implement `Serialize`/`Deserialize` and round-trip JSON tests.
- [x] Linter guardrails in `crates/runie-core/build.rs` still pass.

## Tests

### Layer 1 — State / Logic

- 12 unit tests in `crates/runie-core/src/orchestrator.rs`
  - Task status valid/invalid transitions
  - `plan_serializes_round_trip` (JSON round-trip with all fields)
  - `plan_simple_helper`, `plan_completion_helpers`
  - `subagent_task_builder`, `subagent_task_not_runnable_when_not_pending`
  - `model_trait_labels`, `model_trait_display`
  - `plan_result_serializes`, `plan_result_with_failures`

## Files touched

- `crates/runie-core/src/orchestrator.rs` (new — 456 lines)
- `crates/runie-core/src/lib.rs` (added `pub mod orchestrator`)

## Out of scope

- LLM prompt construction.
- Actor runtime wiring.
- Subagent execution.
