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
- The unwired `UiControlActor` subtree (`crates/runie-core/src/actors/ui_control/`). It is not included in `actors/mod.rs`, so it does not affect compilation, but it should still be removed.
- Dead fields/helpers in `ActorHandles` (`view`, `completion`, `trust`, `send_view`, `send_trust`, `send_init_read_only`, etc.)

## Acceptance Criteria

- [ ] `crates/runie-core/src/actors/trait.rs` is deleted and no production code references its types.
- [ ] Both legacy and ractor implementations of `ViewActor`, `PlanActor`, `TrustActor`, `CompletionActor`, and `UiControlActor` are deleted.
- [ ] `crates/runie-core/src/actors/mod.rs` exports only ractor-based production actor types.
- [ ] `crates/runie-core/src/actors/handles.rs` no longer contains fields or helpers for deleted actors.
- [ ] `Reply` is moved out of `trait.rs` to `actors/ractor_adapter.rs` or a new `actors/reply.rs`, and all callers are updated before `trait.rs` is deleted.
- [ ] `GenericActorHandle` usage in `SessionActorHandle`, `PersistenceActorHandle`, `SessionStoreActorHandle`, and `TrustActorHandle` is replaced with `ractor::ActorRef` or dedicated mpsc wrappers.
- [ ] `crates/runie-core/src/testing/actor_harness.rs` is updated to use `ractor` or kept under `#[cfg(test)]` without referencing the deleted trait.
- [ ] The non-functional `RactorHandle::rpc` in `crates/runie-core/src/actors/ractor_adapter.rs` is either implemented with a message-carrying reply sender or deleted.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `no_custom_actor_trait_references` — workspace grep shows no imports of `runie_core::actors::{Actor, spawn_actor, GenericActorHandle, Reply}` in production code.
- [ ] `actor_handles_has_no_dead_fields` — `ActorHandles` exposes only production actor refs.

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
- `crates/runie-core/src/actors/ui_control/` (delete)
- `crates/runie-core/src/actors/turn/actor.rs` (delete legacy custom impl, keep `ractor_turn.rs`)
- `crates/runie-core/src/actors/permission/actor.rs` (delete legacy custom impl, keep `ractor_permission.rs`)
- `crates/runie-core/src/actors/mod.rs`
- `crates/runie-core/src/actors/handles.rs`
- `crates/runie-core/src/testing/actor_harness.rs`

## Notes

- This task is purely mechanical once `migrate-production-actors-to-ractor` is complete. If the build breaks, it means a production actor was missed.
- `UiControlActor` references `Event::DialogOpened`, `Event::DialogClosed`, etc., which do not exist. It is not compiled today because `ui_control` is not declared in `actors/mod.rs`. Delete the directory rather than fixing it.
- `Reply` and `GenericActorHandle` are re-exported by `actors/mod.rs` and used by several message modules. Move `Reply` to `actors/ractor_adapter.rs` (or a new `actors/reply.rs`) before deleting `trait.rs`, and migrate callers from `GenericActorHandle` to `ractor::ActorRef`.
- `crates/runie-core/src/actors/ractor_adapter.rs:123` has a non-functional `rpc` method (creates a oneshot receiver but never sends the reply). Delete it or replace it with a message variant carrying an `RpcReply` sender before this task is complete.
- Rejected alternative: keeping the custom trait as a thin wrapper. It adds no value and still requires maintenance.
