# Simplify Actor trait: drop unused default, unify spawn ergonomics

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: unify-persistence-actors
**Blocks**: none

## Description

The `Actor` trait in `crates/runie-core/src/actor.rs` has two friction points:

1. **Unused default `run_body`** (actor.rs:59-70). The default impl is `async move { let _ = (self, rx, bus); }` — a no-op. Every real actor overrides `run_body`. The default exists only to satisfy the trait, adding indirection without value. The `run` method (actor.rs:46-55) wraps `run_body` in a `Box::pin`, but since every actor overrides `run_body`, `run` could directly require the async body and the default `run_body` deleted.

2. **Inconsistent spawn ergonomics.** `spawn_actor` (actor.rs:83) returns `(mpsc::Sender<A::Msg>, ActorHandle)`, but each actor's own `spawn` method returns `(XActorHandle, ActorHandle)` wrapping the sender in a typed handle. The generic `spawn_actor` is never called outside `actor.rs` tests — every actor uses its own `spawn`. The generic helper is dead production code.

Simplify: make `run_body` the only required method (no default), and either delete `spawn_actor` or make every actor use it consistently.

## Acceptance Criteria

- [ ] `Actor::run_body` has no default implementation (required method); OR `run` is removed and `run_body` is the trait method that returns the future.
- [ ] `Actor::run` default wrapper (actor.rs:46-55) either inlined or removed; no `Box::pin` indirection if avoidable.
- [ ] `spawn_actor` generic helper either deleted (if every actor uses its own `spawn`) OR used by every actor (pick one pattern).
- [ ] `ActorHandle` kept (abort + join is useful); its `Drop` abort behavior preserved.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `actor_trait_has_no_noop_default` — grep assertion: `run_body` has no `let _ = (self, rx, bus)` default body.

### Layer 2 — Event Handling
- [ ] `test_actor_still_runs_and_receives_messages` — the existing `actor.rs` test actor still functions after trait simplification.
- [ ] `actor_supervision_cancels_on_drop` — `ActorHandle` drop-abort behavior preserved.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_all_actors_spawn_after_trait_simplification` — TUI + headless bootstrap spawn all actors successfully.

## Files touched

- `crates/runie-core/src/actor.rs` — simplify trait, remove dead default/spawn_actor
- All actor files (`actors/config/`, `actors/io/`, `actors/session/`, `actors/provider/`, `actors/fff_indexer/`, `runie-agent/src/actor.rs`) — update if `spawn_actor` signature changes

## Notes

Depends on `unify-persistence-actors` so the actor count is stable before simplifying the trait (the unified `SessionActor` is the last actor to settle). This is low-risk: the trait has exactly 6 implementors and all override `run_body`. The `inline-thin-abstractions` task notes that `SessionActor` (pre-unification) has `type Msg = ()` — after `unify-persistence-actors` it receives real messages, so the `Actor` trait is fully justified for all implementors. Keep this small and mechanical.
