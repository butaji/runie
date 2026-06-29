# Actually collapse `ActorHandles` to a typed map

**Status**: todo
**Milestone**: R6
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: delete-dead-actor-handle-wrappers
**Blocks**: migrate-tui-and-cli-to-leader-bootstrap

## Description

`ActorHandles` is a 300-line custom façade with `Option<Ractor*Handle>` fields and per-actor delegation helpers. The earlier task collapsed the old handle map but left this custom struct. Replace it with a small typed map or struct of `ractor::ActorRef<Msg>` / thin newtypes, deleting all delegation methods.

## Acceptance Criteria

- [ ] Replace `ActorHandles` with a struct of `ractor::ActorRef<Msg>` fields (one per production actor).
- [ ] Delete all delegation helper methods; callers use `actor_ref.cast(...)` or `call!` directly.
- [ ] Remove `Option` wrappers where an actor is always present.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `handles_hold_actor_refs` — every field is a concrete `ActorRef`.
- [ ] `handles_no_delegation_methods` — no helper methods remain.

### Layer 2 — Event Handling
- [ ] `handle_cast_reaches_actor` — casting via the typed map delivers the message.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `typed_map_turn_completes` — a turn submitted through the typed map completes.

## Files touched

- `crates/runie-core/src/actors/handles.rs`
- `crates/runie-core/src/actors/handles_tests.rs`
- `crates/runie-core/src/actors/mod.rs`
- `crates/runie-core/src/actors/leader/actor.rs`
- `crates/runie-tui/src/main.rs`
- `crates/runie-cli/src/acp.rs`
- `crates/runie-agent/src/actor.rs`

## Notes

- The previous `collapse-actor-handles-to-typed-map.md` task left a façade; this task finishes the job.
- Coordinate with `delete-dead-actor-handle-wrappers.md`.
