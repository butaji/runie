# Replace custom actor runtime with `ractor`

**Status**: done
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

### Phase 2: Actor Migration (COMPLETE)
- [x] InputActor migrated to ractor
- [x] PermissionActor migrated to ractor
- [x] ViewActor migrated to ractor
- [x] CompletionActor migrated to ractor
- [x] TrustActor migrated to ractor
- [x] UiControlActor migrated to ractor
- [x] PlanActor migrated to ractor
- [x] TurnActor migrated to ractor
- [x] Event-bus integration and request/response patterns preserved
- [x] Updated Leader to use ractor-based actors

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
- [x] Remaining actors migrated incrementally to ractor.
- [x] Event-bus integration and request/response patterns preserved.

## Tests

- **Layer 2**: Actor message handling and lifecycle tests.
- **Layer 4**: Agent-turn replay tests that exercise actor interactions.

## Files Changed

- `Cargo.toml` - Added `ractor = "0.9"` to workspace dependencies
- `crates/runie-core/Cargo.toml` - Added ractor dependency
- `crates/runie-core/src/actors/ractor_adapter.rs` - Adapter layer
- `crates/runie-core/src/actors/mod.rs` - Export ractor adapter module
- `crates/runie-core/src/actors/input/actor.rs` - InputActor migrated to ractor
- `crates/runie-core/src/actors/input/ractor_input.rs` - Ractor-based InputActor
- `crates/runie-core/src/actors/permission/ractor_permission.rs` - Ractor-based PermissionActor
- `crates/runie-core/src/actors/permission/mod.rs` - Export ractor-based PermissionActor
- `crates/runie-core/src/actors/view/ractor_view.rs` - Ractor-based ViewActor
- `crates/runie-core/src/actors/view/mod.rs` - Export ractor-based ViewActor
- `crates/runie-core/src/actors/completion/ractor_completion.rs` - Ractor-based CompletionActor
- `crates/runie-core/src/actors/completion/mod.rs` - Export ractor-based CompletionActor
- `crates/runie-core/src/actors/trust/ractor_trust.rs` - Ractor-based TrustActor
- `crates/runie-core/src/actors/trust/mod.rs` - Export ractor-based TrustActor
- `crates/runie-core/src/actors/ui_control/ractor_ui_control.rs` - Ractor-based UiControlActor
- `crates/runie-core/src/actors/ui_control/mod.rs` - Export ractor-based UiControlActor
- `crates/runie-core/src/actors/plan/ractor_plan.rs` - Ractor-based PlanActor
- `crates/runie-core/src/actors/plan/mod.rs` - Export ractor-based PlanActor
- `crates/runie-core/src/actors/turn/ractor_turn.rs` - Ractor-based TurnActor
- `crates/runie-core/src/actors/turn/mod.rs` - Export ractor-based TurnActor
- `crates/runie-core/src/actors/leader/actor.rs` - Updated to use Ractor actors
- `crates/runie-core/src/actors/handles.rs` - Updated to use RactorTurnHandle
- `crates/runie-tui/Cargo.toml` - Added ractor dependency
- `crates/runie-tui/src/main.rs` - Updated to use ractor-based actors
- `crates/runie-tui/src/ui_actor.rs` - Updated to use RactorTurnHandle
- `crates/runie-agent/src/actor.rs` - Updated to use RactorTurnHandle in tests

## Completed Migrations

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

## Notes

The migration was done incrementally:
1. Added ractor as a dependency
2. Created a thin adapter layer (`ractor_adapter.rs`) that:
   - Provides `RactorHandle<Msg>` as a cloneable actor reference
   - Provides `spawn_ractor()` function similar to existing `spawn_actor()`
   - Provides `EventBusBridge<E>` for publishing to the shared EventBus
   - Provides `RpcReply<T>` for request/response patterns
   - Includes tests for all new functionality
3. Each actor was migrated by creating `ractor_<actor>.rs` with ractor-based implementation
4. Updated `mod.rs` to export the new ractor handle type
5. Type aliased the old handles to the new ractor handles for backward compatibility
6. Updated all callers (Leader, TUI, Agent) to use the new ractor-based actors

### Migration Pattern
Each actor migration followed this pattern:
1. Create `ractor_<actor>.rs` with ractor-based implementation
2. Update `mod.rs` to export the new ractor handle type
3. Type alias the old handle to the new ractor handle for backward compatibility
4. Add tests for the new implementation
5. Update all production callers to use the new ractor-based actor
