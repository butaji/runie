# Actor Lifecycle and Handle Registry

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: actor-owned-state-ssot
**Blocks**: test-actor-harness, env-actor-owns-git-cwd, fff-indexer-owns-file-picker-results, remove-direct-appstate-mutations

## Description

Document and implement the actor lifecycle management and handle registry pattern. Each actor should have a clean spawn/start/stop lifecycle, and `ActorHandles` should be the single entry point for sending messages to actors.

## Acceptance Criteria

- [ ] `ActorHandles` struct documented with all actor handles
- [ ] Clean actor spawn pattern defined
- [ ] Actor shutdown/cleanup documented
- [ ] No dangling handles or zombie actors
- [ ] `cargo test --workspace` passes

## Tests

### Layer 1 — State/Logic
- [ ] `actor_handle_spawns_correctly`
- [ ] `actor_handle_aborts_on_drop`

### Layer 2 — Event Handling
- [ ] N/A

### Layer 3 — Rendering
- [ ] N/A

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A

## Files touched

- `crates/runie-core/src/actors/handles.rs`
- `crates/runie-core/src/actors/trait.rs`

## Notes

- This is a documentation/implementation refinement task
- The actor infrastructure already exists in `actors/trait.rs`
