# Adopt or Remove Actor Framework

**Status**: todo
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P1

**Depends on**: (none)
**Blocks**: (none)

## Description

Earlier design documents and some in-code comments describe a typed actor
runtime (`Actor` trait, `EventBus`, `InputActor`, `AgentActor`, etc.).
In practice, `runie-term/src/main.rs` uses ad-hoc tokio tasks and direct
channels. The `EventBus` in `runie-core/src/bus.rs` and related actor
abstractions may be dead code or only partially wired.

Runie must either commit to the actor abstraction and integrate it, or
delete it and document the simpler task/channel model that is actually in
use.

## Acceptance Criteria

- [ ] Decision recorded in an ADR update or new ADR.
- [ ] If adopting: `EventBus`, `Actor` trait, and the actor types are fully
  integrated into `runie-term` and replace ad-hoc channels.
- [ ] If removing: `EventBus`, `Actor` trait, and related dead modules are
  deleted; `runie-term` channel plumbing is documented.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `event_bus_or_deleted` — either `EventBus` has active subscribers in
  production code, or it no longer exists.

### Layer 2 — Event Handling
- [ ] `input_actor_publishes_events` (if adopted) — `InputActor` publishes
  events through the bus.

## Files touched

- `crates/runie-core/src/bus.rs`
- `crates/runie-core/src/session_actor.rs`
- `crates/runie-term/src/main.rs`
- `docs/adr/0017-actor-runtime-and-event-bus.md`
- `docs/SPEC.md`

## Notes

The lightweight tokio-task direction from `tasks/actor-runtime-decision.md`
is the preferred default. This task is about making the codebase consistent
with that decision.
