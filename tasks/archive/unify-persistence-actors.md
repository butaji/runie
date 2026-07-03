# Unify Persistence Actors

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: event-taxonomy-for-actor-state-sync
**Blocks**: simplify-actor-trait

## Summary

Defined a `PersistenceActor` trait that documents the shared load/persist lifecycle
contract for actors owning durable state. Both `ConfigActor` and `SessionActor`
now implement it, with `load_all()` called at the start of `run_body()` before
the message loop.

## What was implemented

- `crates/runie-core/src/actors/persistence.rs` — trait definition
- `ConfigActor::load_all` → delegates to existing `load_and_emit`
- `SessionActor::load_all` → refactored to take `&EventBus<Event>` param; emits `TrustLoaded` and `HistoryLoaded` facts
- `PersistenceActor` re-exported from `actors::` and `lib.rs`

## Acceptance Criteria

- [x] Common persistence trait defined
- [x] `SessionActor` and `ConfigActor` implement it
- [x] `cargo test --workspace` passes

## Tests

### Layer 1 — State/Logic
- [x] `persistence_trait_implemented` — compile-time assertion that both actors implement `PersistenceActor`

### Layer 2 — Event Handling
- N/A

### Layer 3 — Rendering
- N/A

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `actor_loads_and_emits_trust_and_history` — existing session test verifies `SessionActor::spawn` → `load_all` → fact emission chain
- [x] `config_actor_loads_and_emits_config_loaded` — existing config test verifies `ConfigActor::spawn` → `load_all` → fact emission chain
