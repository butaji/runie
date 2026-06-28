# Consolidate actor runtime on `ractor`

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: none
**Blocks**: centralize-runtime-bootstrap-in-leaderactor

## Description

Replace the remaining custom actor runtime in `runie-core` with a single `ractor`-based runtime, deleting the bespoke `Actor` trait, `spawn_actor` helper, and any actors that are only spawned in tests. Refactor `ActorHandles` from a broad bag of per-actor helpers into a small, typed map of `ractor::ActorRef` keyed by the production actor set.

## Acceptance Criteria

- [ ] Delete `crates/runie-core/src/actors/trait.rs` and all references to the custom `Actor`, `spawn_actor`, `GenericActorHandle`, and `Reply` abstractions.
- [ ] Delete `ViewActor`, `PlanActor`, `TrustActor`, `UiControlActor`, `CompletionActor`, and both their custom and `Ractor*` counterparts, since none are spawned in production.
- [ ] Keep only `ConfigActor`, `ProviderActor`, `IoActor`, `SessionActor`, `RactorPermissionActor`, `RactorTurnActor`, and `InputActor` as first-class runtime entities.
- [ ] Collapse `crates/runie-core/src/actors/handles.rs` from per-actor helper methods to a typed map of `ractor::ActorRef<ActorType>`.
- [ ] Update all production spawn sites to use the collapsed handle set.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `actor_handles_contains_only_production_actors` — verifies that the collapsed `ActorHandles` exposes exactly the expected typed actor refs and no dead fields remain.

### Layer 2 — Event Handling
- [ ] `ractor_actor_spawn_lifecycle` — starts and stops each production actor through `ractor` and asserts clean shutdown with no orphan `ActorRef`s.

### Layer 3 — Rendering
- [ ] N/A — this task removes runtime plumbing; no TUI widgets or render buffers are changed.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `only_expected_actors_exist` — runs an end-to-end startup smoke test, reflects over the live `ActorHandles`, and confirms only the whitelisted production actors are registered.

## Files touched

- `crates/runie-core/src/actors/trait.rs` (delete)
- `crates/runie-core/src/actors/handles.rs`
- `crates/runie-core/src/actors/mod.rs`
- `crates/runie-core/src/actors/leader/actor.rs`
- `crates/runie-core/src/actors/config.rs`
- `crates/runie-core/src/actors/provider.rs`
- `crates/runie-core/src/actors/io.rs`
- `crates/runie-core/src/actors/session.rs`
- `crates/runie-core/src/actors/permission.rs`
- `crates/runie-core/src/actors/turn.rs`
- `crates/runie-core/src/actors/input.rs`
- `crates/runie-tui/src/main.rs`
- `crates/runie-cli/src/acp.rs`

## Notes

The custom trait was an early abstraction that is now fully subsumed by `ractor`. `ViewActor`, `PlanActor`, `TrustActor`, `UiControlActor`, and `CompletionActor` were kept for exploratory tests but have no production callers; their removal simplifies the runtime surface and eliminates dead code from the build. Rejected alternative: keeping the trait as a thin wrapper around `ractor::ActorRef` — it adds no value and still requires maintenance. Out of scope: changing actor message protocols; this task is purely about runtime consolidation and handle cleanup.
