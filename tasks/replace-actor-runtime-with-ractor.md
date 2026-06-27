# Replace custom actor runtime with `ractor`

**Status**: in_progress
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: actor-owned-state-ssot
**Blocks**: none

## Summary

Replace the home-grown `Actor` trait, `spawn_actor`, `ActorHandle`, and `Reply` wrapper with the `ractor` framework. Retain actor-owned SSOT semantics and the event bus.

## Progress

- [x] `ractor` added to workspace dependencies
- [x] Created `ractor_adapter.rs` with thin wrapper layer
- [x] Created `ractor_input.rs` with proof-of-concept migration of InputActor
- [x] Ractor-based InputActor spawns correctly
- [x] Ractor-based InputActor receives messages
- [ ] Migrate remaining actors to ractor (can proceed incrementally)
- [ ] Update task list to reflect progress

## Acceptance Criteria

- [x] `ractor` is added to workspace dependencies.
- [x] `crates/runie-core/src/actors/ractor_adapter.rs` provides thin wrapper layer.
- [x] Proof-of-concept: InputActor can spawn and receive messages via ractor.
- [ ] Remaining actors migrated incrementally to ractor.
- [ ] Event-bus integration and request/response patterns preserved.
- [ ] `cargo check --workspace` is green with no new warnings.

## Tests

- **Layer 2**: Actor message handling and lifecycle tests.
- **Layer 4**: Agent-turn replay tests that exercise actor interactions.

## Files Changed

- `Cargo.toml` - Added `ractor = "0.9"` to workspace dependencies
- `crates/runie-core/Cargo.toml` - Added ractor dependency
- `crates/runie-core/src/actors/ractor_adapter.rs` - New adapter layer
- `crates/runie-core/src/actors/mod.rs` - Added ractor_adapter module

## Notes

The migration is being done incrementally:
1. Added ractor as a dependency
2. Created a thin adapter layer (`ractor_adapter.rs`) that:
   - Provides `RactorHandle<Msg>` as a cloneable actor reference
   - Provides `spawn_ractor()` function similar to existing `spawn_actor()`
   - Provides `EventBusBridge<E>` for publishing to the shared EventBus
   - Provides `RpcReply<T>` for request/response patterns
   - Includes tests for all new functionality
3. Existing actors remain unchanged; they can be migrated one at a time
