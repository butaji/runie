# One-Shot Orchestrator LLM Planner

**Status**: done
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

## What was implemented

- `crates/runie-provider/src/planner.rs` (new)
  - `PlannerConfig` — `max_retries` (default 2), `timeout` (default 60s)
  - `PlannerError` — `Timeout`, `ParseFailed`, `ValidationFailed`, `NoModelForTrait`, `ProviderError`
  - `ToolDescription`, `ProjectContext`, `PlanInput`
  - `build_planner_system_prompt()` — includes traits, tools, JSON schema, rules
  - `build_user_prompt()` — project context, directories, key files, dialogue Q&A
  - `parse_raw_plan()` — maps raw JSON → `OrchestratorPlan`, validates traits and tools
  - `extract_json_from_text()` — strips markdown code fences before parsing
  - `OneShotPlanner<P>` — async planner with retry loop, timeout, markdown extraction
  - `validate_plan()` — checks for duplicate IDs, resolves all model traits
- `crates/runie-provider/src/fixtures/plan.json` — valid test fixture
- `crates/runie-provider/src/fixtures/plan_bad_trait.json` — invalid trait fixture

## Acceptance Criteria

- [x] Planner prompt includes system instructions, available traits, available
  tools, project context, and the user request.
- [x] LLM output is parsed into `OrchestratorPlan` with robust error handling.
- [x] Failed parse triggers a retry with a clearer prompt up to `N` times
  (configurable, default 2).
- [x] Plan validation ensures each task references only available tools and a
  resolvable model trait.
- [x] Planner is deterministic enough to be tested with a mock provider.
- [x] `cargo test --workspace` passes.

## Tests (7 total)

### Layer 1 — State / Logic

- `planner_parses_valid_json` — MockProvider returns plan.json → 2 tasks
- `planner_retries_on_invalid_json` — invalid then valid → 2 tasks after retry
- `planner_retries_on_invalid_trait` — bad trait then valid → 2 tasks after retry
- `planner_extracts_json_from_markdown` — JSON in code fence → 2 tasks
- `plan_validation_rejects_unknown_trait` — duplicate ID → ValidationFailed
- `orchestrator_context_included_in_prompt` — Q&A appears in prompt

### Layer 2 — Event Handling

- Covered by `orchestrator_context_included_in_prompt` (prompt construction)

## Files touched

- `crates/runie-provider/src/planner.rs` (new)
- `crates/runie-provider/src/fixtures/plan.json` (new)
- `crates/runie-provider/src/fixtures/plan_bad_trait.json` (new)
- `crates/runie-provider/src/lib.rs` — added `pub mod planner`

## Out of scope

- Multi-shot refinement loop.
- Recursive planning (future).
