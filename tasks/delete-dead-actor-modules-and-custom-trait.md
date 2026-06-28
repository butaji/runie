# Delete dead actor modules and the custom `Actor` trait

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: migrate-production-actors-to-ractor
**Blocks**: collapse-actor-handles-to-typed-map

## Description

Once every production actor runs on `ractor`, the legacy custom `Actor` trait and the actors that are only used in tests become dead code. This task removes:

- `crates/runie-core/src/actors/trait.rs` (custom `Actor`, `spawn_actor`, `ActorHandle`, `GenericActorHandle`, `Reply`)
- `ViewActor` and its ractor counterpart (`crates/runie-core/src/actors/view/`)
- `PlanActor` and its ractor counterpart (`crates/runie-core/src/actors/plan/`)
- `TrustActor` and its ractor counterpart (`crates/runie-core/src/actors/trust/`)
- `CompletionActor` and its ractor counterpart (`crates/runie-core/src/actors/completion/`)
- The legacy custom `TurnActor` and `PermissionActor` implementations, keeping only the ractor-based `RactorTurnActor` and `RactorPermissionActor`
- The broken, unwired `UiControlActor` subtree (`crates/runie-core/src/actors/ui_control/`) unless a separate task decides to revive and fix it

## Acceptance Criteria

- [ ] `crates/runie-core/src/actors/trait.rs` is deleted and no code references its types.
- [ ] All actor modules listed above are deleted.
- [ ] `crates/runie-core/src/actors/mod.rs` exports only ractor-based production actor types.
- [ ] Test-only actor harnesses are updated to use `ractor` or gated with `#[cfg(test)]`.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `no_custom_actor_trait_references` — workspace grep shows no imports of `runie_core::actors::{Actor, spawn_actor, GenericActorHandle, Reply}` in production code.

### Layer 2 — Event Handling
- [ ] N/A — deletion task; no new event routing.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `smoke_no_dead_actors_in_runtime` — starts the runtime and reflects over the actor set to confirm no deleted actor types are registered.

## Files touched

- `crates/runie-core/src/actors/trait.rs` (delete)
- `crates/runie-core/src/actors/view/` (delete)
- `crates/runie-core/src/actors/plan/` (delete)
- `crates/runie-core/src/actors/trust/` (delete)
- `crates/runie-core/src/actors/completion/` (delete)
- `crates/runie-core/src/actors/ui_control/` (delete or fix in a separate task)
- `crates/runie-core/src/actors/turn/actor.rs` (delete legacy custom impl, keep `ractor_turn.rs`)
- `crates/runie-core/src/actors/permission/actor.rs` (delete legacy custom impl, keep `ractor_permission.rs`)
- `crates/runie-core/src/actors/mod.rs`
- `crates/runie-core/src/testing/actor_harness.rs` (if it uses the custom trait)

## Notes

- This task is purely mechanical once `migrate-production-actors-to-ractor` is complete. If the build breaks, it means a production actor was missed.
- `UiControlActor` references `Event::DialogOpened`, `Event::DialogClosed`, etc., which do not exist. Rather than fixing it here, delete it and let a future task reintroduce a working UI-control actor if needed.
- Rejected alternative: keeping the custom trait as a thin wrapper. It adds no value and still requires maintenance.
