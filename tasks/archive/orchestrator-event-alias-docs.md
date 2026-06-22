# Document OrchestratorEvent Alias

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P3

**Depends on**: none
**Blocks**: none

## Description

`pub type OrchestratorEvent = crate::Event;` flattens orchestrator events into the global enum, which is intentional per R3 but can be confusing when the same type is used for two buses.

## Acceptance Criteria

- [ ] Add a doc comment explaining the alias and the forward-mapping done in `forward_orchestrator_events`.
- [ ] `cargo test --workspace` succeeds.

## Tests

N/A — documentation only.

## Files touched

- `crates/runie-core/src/orchestrator_actor.rs`

## Notes

Low-risk documentation improvement.
