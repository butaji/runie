# Replace custom actor runtime with `ractor`

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: actor-owned-state-ssot
**Blocks**: none

## Summary

Replace the home-grown `Actor` trait, `spawn_actor`, `ActorHandle`, and `Reply` wrapper with the `ractor` framework. Retain actor-owned SSOT semantics and the event bus.

## Acceptance Criteria

- `ractor` is added to workspace dependencies.
- `crates/runie-core/src/actors/trait.rs`, `actors/handles.rs`, and per-actor message boilerplate are removed or simplified.
- All existing actors run under `ractor` with equivalent lifecycle and supervision.
- Event-bus integration and request/response patterns are preserved.
- `cargo check --workspace` is green with no new warnings.

## Tests

- **Layer 2**: Actor message handling and lifecycle tests.
- **Layer 4**: Agent-turn replay tests that exercise actor interactions.
