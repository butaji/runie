# Orchestrator Actor

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: actor-runtime-decision, r4-orchestrator-domain-types, r4-one-shot-orchestrator-llm
**Blocks**: r4-subagent-isolation, r4-subagent-sidebar, r4-team-mode-integration

## Description

Create an `OrchestratorActor` that lives in the actor runtime, owns the
Orchestrator state machine, and coordinates subagent execution. It receives
user requests, runs the one-shot planner, dispatches subagent tasks, collects
results, runs synthesis, and emits feed events.

## What was implemented

- `crates/runie-core/src/orchestrator_actor.rs` (new)
  - `OrchestratorState` — `Idle`, `Aligning`, `Planning`, `Executing`, `Synthesizing`,
    `Done`, `Failed`
  - `OrchestratorCommand` — `StartRequest`, `UserAnswer`, `SubagentStatusUpdate`,
    `SubagentDone`, `SubagentFailed`, `Cancel`, `Reset`
  - `OrchestratorEvent` — `StateChanged`, `PlanningStarted`, `PlanGenerated`,
    `PlanningFailed`, `SubagentDispatched`, `SubagentStatusChanged`,
    `SubagentCompleted`, `SubagentFailed`, `SynthesisStarted`, `SynthesisComplete`,
    `Finished`, `Cancelled`
  - `OrchestratorActor` — implements `Actor` trait, drives state machine,
    command handlers split into small functions for lint compliance
  - `OrchestratorContext` integration — `record_question` / `record_answer`
  - `has_pending_questions()` / `can_submit_plan()` — blocks submission until answered
- `crates/runie-core/src/orchestrator.rs` — added `PartialEq` to `SubagentTask`,
  `OrchestratorPlan`, `PlanResult`, `TaskFailure` for state comparison
- `crates/runie-core/src/lib.rs` — `pub mod orchestrator_actor`

## Acceptance Criteria

- [x] `OrchestratorActor` implements the actor `Handle` trait (implements `Actor`).
- [x] States: `Idle`, `Aligning`, `Planning`, `Executing`, `Synthesizing`,
  `Done`, `Failed`.
- [x] Emits events for state transitions, subagent status changes, and final
  synthesis.
- [x] Persists its state to the event bus (via `OrchestratorEvent::StateChanged`).
- [x] Handles cancellation gracefully when the user switches back to Solo mode
  or aborts.
- [x] `cargo test --workspace` passes.

## Tests (Layer 1 — State/Logic)

- `actor_starts_idle` — default state is `Idle`
- `start_request_transitions_to_aligning` — can start request when idle
- `record_question_marks_pending` — question sets pending flag
- `record_answer_clears_pending` — answer clears pending flag
- `cancel_resets_to_idle` — cancel clears state and plan
- `collect_subagent_result` — results accumulated correctly
- `orchestrator_state_is_terminal` — Done/Failed/Idle are terminal

## Files touched

- `crates/runie-core/src/orchestrator_actor.rs` (new)
- `crates/runie-core/src/orchestrator.rs` — added PartialEq derives
- `crates/runie-core/src/lib.rs` — `pub mod orchestrator_actor`

## Out of scope

- Subagent actor implementation (r4-subagent-isolation).
- Sidebar UI (r4-subagent-sidebar).
