# Remove dead migration scaffolding

**Status**: todo
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P3

**Depends on**: actually-collapse-actor-handles-to-typed-map
**Blocks**: none

## Description

Delete leftover scaffolding from the actor migration: `RactorActor` wrapper, `ActorHandles` alias for `LeaderHandle`, deprecated `InputActorHandle`, and duplicate `RactorHandle::send_message` alias.

## Acceptance Criteria

- [ ] Delete `RactorActor` from `ractor_adapter.rs`.
- [ ] Delete `ActorHandles` alias or keep only as a deprecated re-export.
- [ ] Delete deprecated `InputActorHandle`.
- [ ] Remove duplicate `send_message` alias on `RactorHandle`.
- [ ] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [ ] `dead_scaffolding_removed` — grep confirms aliases/wrappers gone.

## Files touched

- `crates/runie-core/src/actors/ractor_adapter.rs`
- `crates/runie-core/src/actors/handles.rs`
- `crates/runie-core/src/actors/input/mod.rs`

## Notes

- Low priority; good cleanup before declaring the migration finished.
