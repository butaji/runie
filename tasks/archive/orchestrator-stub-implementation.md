# Orchestrator Actor Stub Implementation

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

`OrchestratorActor::handle_start_request` and `handle_user_answer` transition `Aligning → Planning → Executing` without any actual LLM planner call or subagent dispatch. The comment says "Async planner call goes in r4-subagent-execution; stub here". This means Team mode currently produces no real plan and no real subagent work.

## Acceptance Criteria

- [ ] Either implement the planner invocation and subagent dispatch loop, or explicitly gate Team mode behind a "not implemented" warning/error.
- [ ] State transitions are justified by real outcomes, not no-ops.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `team_mode_planner_produces_tasks` — planner returns a list of subagent tasks.
- [ ] `orchestrator_spawns_subagents_for_tasks` — subagents are spawned and tracked.

### Layer 2 — Event Handling
- [ ] `orchestrator_forwards_subagent_events` — subagent events reach the UI.

### Layer 3 — Rendering
N/A — covered by existing subagent/UI rendering.

### Layer 4 — Smoke / Crash
- [ ] `smoke_team_mode_shows_planning` — Team mode does not silently no-op.

## Files touched

- `crates/runie-core/src/orchestrator_actor.rs`

## Notes

If the full R4 subagent pipeline is not ready, a minimal safe fix is to make Team mode return an informative error and fall back to Solo mode until implementation is complete.
