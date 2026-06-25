# Actor Lifecycle and Handle Registry

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: actor-owned-state-ssot
**Blocks**: test-actor-harness, env-actor-owns-git-cwd, fff-indexer-owns-file-picker-results, remove-direct-appstate-mutations

## Description

Document and implement the actor lifecycle management and handle registry pattern. Each actor should have a clean spawn/start/stop lifecycle, and `ActorHandles` should be the single entry point for sending messages to actors.

## Progress (2026-06-25)

The actor lifecycle infrastructure is already implemented:

- ✅ `ActorHandles` struct with all actor handles (config, provider, session, io, fff_indexer)
- ✅ Typed `send_*` helper methods for each actor message
- ✅ `FffIndexerHandle` dedicated handle type
- ✅ `ActorHandle` in `trait.rs` provides spawn, abort, and drop semantics
- ✅ `Reply<T>` generic reply wrapper for request/response patterns
- ✅ Tests verify default state and integration

## Acceptance Criteria

- [x] `ActorHandles` struct documented with all actor handles
- [x] Clean actor spawn pattern defined
- [x] Actor shutdown/cleanup documented (via `ActorHandle::abort_on_drop`)
- [x] No dangling handles or zombie actors (drop semantics in place)
- [x] `cargo test --workspace` passes

## Tests

### Layer 1 — State/Logic
- [x] `actor_handles_default_is_all_none` — default state is unit-test friendly
- [x] `actor_handles_send_save_provider_via_actor` — integration test

### Layer 2 — Event Handling
- N/A

### Layer 3 — Rendering
- N/A

### Layer 4 — Provider Replay / Mock-Tool E2E
- N/A

## Files touched

- `crates/runie-core/src/actors/handles.rs`
- `crates/runie-core/src/actors/trait.rs`

## Notes

- The actor infrastructure is well-documented and tested
- `ActorHandle` provides spawn, abort, and drop semantics
- `Reply<T>` provides generic request/response wrapper
