# Replace custom actor runtime with `ractor`

**Status**: todo-progress
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
- [x] ViewActor migrated to ractor
- [x] CompletionActor migrated to ractor
- [x] TrustActor migrated to ractor
- [x] UiControlActor migrated to ractor
- [x] PlanActor migrated to ractor
- [x] TurnActor migrated to ractor
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
- [x] ViewActor migrated to ractor.
- [x] CompletionActor migrated to ractor.
- [x] TrustActor migrated to ractor.
- [x] UiControlActor migrated to ractor.
- [x] PlanActor migrated to ractor.
- [x] TurnActor migrated to ractor.
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
- `crates/runie-core/src/actors/view/ractor_view.rs` - New ractor-based ViewActor
- `crates/runie-core/src/actors/view/mod.rs` - Export ractor-based ViewActor
- `crates/runie-core/src/actors/completion/ractor_completion.rs` - New ractor-based CompletionActor
- `crates/runie-core/src/actors/completion/mod.rs` - Export ractor-based CompletionActor
- `crates/runie-core/src/actors/trust/ractor_trust.rs` - New ractor-based TrustActor
- `crates/runie-core/src/actors/trust/mod.rs` - Export ractor-based TrustActor
- `crates/runie-core/src/actors/ui_control/ractor_ui_control.rs` - New ractor-based UiControlActor
- `crates/runie-core/src/actors/ui_control/mod.rs` - Export ractor-based UiControlActor
- `crates/runie-core/src/actors/plan/ractor_plan.rs` - New ractor-based PlanActor
- `crates/runie-core/src/actors/plan/mod.rs` - Export ractor-based PlanActor
- `crates/runie-core/src/actors/turn/ractor_turn.rs` - New ractor-based TurnActor
- `crates/runie-core/src/actors/turn/mod.rs` - Export ractor-based TurnActor
- `crates/runie-core/src/actors/leader/actor.rs` - Updated to use RactorPermissionActor
- `crates/runie-tui/src/main.rs` - Updated to use RactorPermissionActor
- `crates/runie-cli/src/acp.rs` - Updated to use RactorPermissionActor
- `crates/runie-agent/src/actor.rs` - Updated to use RactorPermissionActor in tests

## Remaining Actors to Migrate

The following actors still use the custom runtime and can be migrated incrementally:

1. ConfigActor - owns config state and file IO (complex: file watcher thread)
2. SessionActor - owns session state and durability
3. IoActor - owns file/network/process operations (complex: async effects)
4. TurnActor - owns agent turn lifecycle
5. ProviderActor - owns provider construction
6. FffIndexerActor - owns file search index
7. PlanActor - owns plan state
8. Leader - needs to be updated to use all ractor actors

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

### Migration Pattern
Each actor migration follows this pattern:
1. Create `ractor_<actor>.rs` with ractor-based implementation
2. Update `mod.rs` to export the new ractor handle type
3. Type alias the old handle to the new ractor handle for backward compatibility
4. Add tests for the new implementation

### Completed Migrations

| Actor | File | Status |
|-------|------|--------|
| InputActor | `ractor_input.rs` | COMPLETE |
| PermissionActor | `ractor_permission.rs` | COMPLETE |
| ViewActor | `ractor_view.rs` | COMPLETE |
| CompletionActor | `ractor_completion.rs` | COMPLETE |
| TrustActor | `ractor_trust.rs` | COMPLETE |
| UiControlActor | `ractor_ui_control.rs` | COMPLETE |
| PlanActor | `ractor_plan.rs` | COMPLETE |
| TurnActor | `ractor_turn.rs` | COMPLETE |

### Pending Migrations

| Actor | Complexity | Notes |
|-------|------------|-------|
| ConfigActor | High | Has file watcher thread |
| SessionActor | Medium | Session persistence |
| IoActor | High | Async effects system |
| ProviderActor | Medium | Provider construction |
| FffIndexerActor | Medium | File search index |
| Leader | Medium | Coordinator actor |
