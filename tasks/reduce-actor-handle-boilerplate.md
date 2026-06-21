# Reduce actor handle and message boilerplate

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: generic-actor-reply
**Blocks**: none

## Description

Every actor repeats: `pub enum XxxMsg { ... }`, `pub struct XxxActorHandle { tx: mpsc::Sender<XxxMsg> }`, `new`, a `tx` accessor, and a set of async request methods. The core actors are also coupled to `EventBus<Event>` in the same way.

## Acceptance Criteria

- [ ] Either a `#[actor]` proc-macro or a generic `ActorHandle<Msg>` helper removes per-actor handle boilerplate.
- [ ] `ConfigActorHandle`, `ProviderActorHandle`, `SessionStoreActorHandle`, `PersistenceActorHandle`, `IoActorHandle`, and `AgentActorHandle` are simplified or generated.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `generic_handle_sends_message` — generic handle can send a message to a spawned actor.
- [ ] `generated_handle_methods_return_expected_value` — request/response pattern works.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `actor_handles_smoke_test` — all actor handles still function in a live turn.

## Files touched

- `crates/runie-core/src/actors/config/handle.rs`
- `crates/runie-core/src/actors/provider/handle.rs`
- `crates/runie-core/src/actors/session_store/handle.rs`
- `crates/runie-core/src/actors/persistence/handle.rs`
- `crates/runie-core/src/actors/io/handle.rs`
- `crates/runie-core/src/actors/agent/handle.rs`
- New macro or helper module.

## Notes

Prefer a generic helper first; introduce a proc-macro only if the helper cannot express the request/response pattern cleanly.
