# Delete dead actor modules and the custom `Actor` trait

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: migrate-production-actors-to-ractor
**Blocks**: collapse-actor-handles-to-typed-map

## Description

The legacy custom `Actor` trait and all dead actor modules were already removed in prior cleanup work. This task completed the remaining dead-code cleanup:

- Removed dead `IoActorHandle` alias from `crates/runie-core/src/actors/io/messages.rs` (coexisted with `RactorIoHandle`; unused in production).
- Removed dead `InputActorHandle` legacy alias from `crates/runie-core/src/actors/input/messages.rs` (the actual handle is `RactorInputHandle` from `actor.rs`).
- Removed non-functional `RactorHandle::rpc` from `crates/runie-core/src/actors/ractor_adapter.rs` (created a oneshot channel but never sent the reply; unused).

The following were already absent from the codebase (deleted in prior work):
- `trait.rs` (custom `Actor`, `spawn_actor`, `ActorHandle`, `GenericActorHandle`, `Reply`)
- `ViewActor`, `PlanActor`, `TrustActor`, `CompletionActor`, `UiControlActor` modules
- Legacy custom `TurnActor` and `PermissionActor` implementations (only ractor-based versions exist)
- `GenericActorHandle` re-export from `actors/mod.rs` is still needed for `RactorSessionHandle` (production use)

## Acceptance Criteria

- [x] `trait.rs` is deleted and no production code references its types.
- [x] Both legacy and ractor implementations of `ViewActor`, `PlanActor`, `TrustActor`, `CompletionActor`, and `UiControlActor` are deleted.
- [x] `crates/runie-core/src/actors/mod.rs` exports only ractor-based production actor types.
- [x] `crates/runie-core/src/actors/handles.rs` no longer contains fields or helpers for deleted actors.
- [x] `Reply` is in `actors/ractor_adapter.rs` and all callers are updated.
- [x] `GenericActorHandle` usage in production (`RactorSessionHandle`) is preserved; dead aliases removed.
- [x] `crates/runie-core/src/testing/actor_harness.rs` is updated to use `ractor` or kept under `#[cfg(test)]` without referencing the deleted trait.
- [x] Non-functional `RactorHandle::rpc` is deleted.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `no_custom_actor_trait_references` — workspace grep shows no imports of `runie_core::actors::{Actor, spawn_actor, GenericActorHandle}` in production code outside of `ractor_adapter.rs` and `session/messages.rs`.
- [x] `actor_handles_has_no_dead_fields` — `ActorHandles` exposes only production actor refs.

### Layer 2 — Event Handling
- N/A — deletion task; no new event routing.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- N/A.

## Files touched

- `crates/runie-core/src/actors/io/messages.rs` — removed dead `IoActorHandle` alias and impl block
- `crates/runie-core/src/actors/input/messages.rs` — removed dead `InputActorHandle` alias and `GenericActorHandle` import
- `crates/runie-core/src/actors/ractor_adapter.rs` — removed non-functional `rpc` method

## Notes

- Most of the task description items were already done by prior cleanup work. The remaining work was purely mechanical dead-code removal.
- `GenericActorHandle` is still needed for `RactorSessionHandle` (production code in `session/messages.rs`).
- `Reply` (= `RpcReply`) is in `ractor_adapter.rs` and used by session message handling.
