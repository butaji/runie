# Simplify Actor trait: drop unused default, unify spawn ergonomics

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: unify-persistence-actors
**Blocks**: none

## Summary

`Actor::run_body` default noop body removed; `run_body` is now a required method (no default). The `spawn_actor` helper is kept — it is used by `IoActor`, `SessionActor`, and `AgentActor`. `ActorHandle` and its `Drop` abort behavior are preserved.

## Implementation

- `run_body` default body removed: `fn run_body(...) -> impl Future<Output = ()> + Send + 'static;` (required, no `= { ... }`)
- `run` still boxes `run_body` via `Box::pin(self.run_body(rx, bus))` — no `Box::pin` indirection added, no `run_body` duplication
- `spawn_actor` kept as-is (3 actors use it, 4 use typed `spawn`)
- `ActorHandle` kept with abort + join + Drop
- `actor_trait_has_no_noop_default` L1 test added
- `actor_trait_resolves_from_actors_module` smoke test fixed (was using `&dyn Actor` which is not dyn-compatible)

## Acceptance Criteria

- [x] `Actor::run_body` has no default implementation (required method)
- [x] `Actor::run` keeps `Box::pin` indirection (needed for `tokio::spawn`); `spawn_actor` kept
- [x] `spawn_actor` kept (3 production actors use it; others use typed `spawn`)
- [x] `ActorHandle` kept; its `Drop` abort behavior preserved
- [x] `cargo test --workspace` succeeds
- [x] `cargo check --workspace` succeeds with no new warnings

## Tests

### Layer 1 — State/Logic
- [x] `actor_trait_has_no_noop_default` — grep assertion: `run_body` has no `let _ = (self, rx, bus)` default body

### Layer 2 — Event Handling
- [x] `actor_trait_runs_and_receives_messages` — existing test actor still functions after simplification
- [x] `actor_supervision_cancels_on_drop` — `ActorHandle` drop-abort behavior preserved

### Layer 3 — Rendering
- N/A

### Layer 4 — Smoke / Crash
- [x] All 7 actors (Config, Provider, Io, Session, FffIndexer, Permission, Agent) spawn and receive messages after simplification

## Files touched

- `crates/runie-core/src/actors/trait.rs` — removed `run_body` default, updated docs, added L1 test

## Notes

- `spawn_actor` is used by Agent, Session, and Io actors in production; the other 4 actors use typed `spawn`. Keeping `spawn_actor` avoids churn for those 3 while preserving a reusable generic helper.
- `ActorHandle` and its `Drop` behavior are unchanged.
