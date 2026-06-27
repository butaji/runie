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

### Phase 1: Infrastructure (COMPLETE)
- [x] `ractor` added to workspace dependencies
- [x] Created `ractor_adapter.rs` with thin wrapper layer
- [x] Created `ractor_input.rs` with proof-of-concept migration of InputActor
- [x] Ractor-based InputActor spawns correctly
- [x] Ractor-based InputActor receives messages
- [x] `cargo check --workspace` is green with no new warnings

### Phase 2: Actor Migration (IN PROGRESS)
- [x] InputActor migrated to ractor
- [x] PermissionActor migrated to ractor
- [ ] Migrate remaining actors to ractor (can proceed incrementally)
- [ ] Event-bus integration and request/response patterns preserved
- [ ] Update task list to reflect progress

## Acceptance Criteria

- [x] `ractor` is added to workspace dependencies.
- [x] `crates/runie-core/src/actors/ractor_adapter.rs` provides thin wrapper layer.
- [x] Proof-of-concept: InputActor can spawn and receive messages via ractor.
- [x] `cargo check --workspace` is green with no new warnings.
- [x] InputActor fully migrated to ractor.
- [x] PermissionActor fully migrated to ractor.
- [ ] Remaining actors migrated incrementally to ractor.
- [ ] Event-bus integration and request/response patterns preserved.

## Tests

- **Layer 2**: Actor message handling and lifecycle tests.
- **Layer 4**: Agent-turn replay tests that exercise actor interactions.

## Files Changed

- `Cargo.toml` - Added `ractor = "0.9"` to workspace dependencies
- `crates/runie-core/Cargo.toml` - Added ractor dependency
- `crates/runie-core/src/actors/ractor_adapter.rs` - New adapter layer
- `crates/runie-core/src/actors/mod.rs` - Added ractor_adapter module
- `crates/runie-core/src/actors/input/actor.rs` - InputActor migrated to ractor
- `crates/runie-core/src/actors/permission/ractor_permission.rs` - New ractor-based PermissionActor
- `crates/runie-core/src/actors/permission/mod.rs` - Export ractor-based PermissionActor
- `crates/runie-core/src/actors/leader/actor.rs` - Updated to use RactorPermissionActor
- `crates/runie-tui/src/main.rs` - Updated to use RactorPermissionActor
- `crates/runie-cli/src/acp.rs` - Updated to use RactorPermissionActor
- `crates/runie-agent/src/actor.rs` - Updated to use RactorPermissionActor in tests

## Notes

The migration is being done incrementally:
1. Added ractor as a dependency
2. Created a thin adapter layer (`ractor_adapter.rs`) that:
   - Provides `RactorHandle<Msg>` as a cloneable actor reference
   - Provides `spawn_ractor()` function similar to existing `spawn_actor()`
   - Provides `EventBusBridge<E>` for publishing to the shared EventBus
   - Provides `RpcReply<T>` for request/response patterns
   - Includes tests for all new functionality
3. `ractor_input.rs` demonstrates the migration pattern
4. `ractor_permission.rs` demonstrates migration with request/response patterns

### Migration Strategy
1. Start with InputActor (COMPLETE - fully migrated to ractor)
2. PermissionActor migrated to ractor with RactorPermissionHandle wrapper
3. Migrate one actor at a time, ensuring tests pass
4. Maintain backward compatibility during transition

### InputActor Migration Details
- InputActor now uses ractor's Actor trait with async_trait
- State is protected by Mutex for interior mutability in async context
- EventBusBridge integrates with the shared EventBus
- RactorInputHandle provides the same send/try_send interface as the old GenericActorHandle
- All existing tests pass with the new implementation

### PermissionActor Migration Details
- RactorPermissionActor uses ractor's Actor trait with async_trait
- RactorPermissionHandle wraps RactorHandle<PermissionMsg> with convenience methods
- Convenience methods: ask_permission, resolve_permission, cancel_permission, dismiss, and their try_* variants
- Tests verify AskPermission emits PermissionRequest and ResolvePermission emits PermissionResponse
