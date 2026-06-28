# Collapse `ActorHandles` to a typed `ractor::ActorRef` map

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: delete-dead-actor-modules-and-custom-trait
**Blocks**: expand-leader-start-for-tui-and-cli, collapse-event-intent-kind-taxonomies

## Description

`crates/runie-core/src/actors/handles.rs` is a 473-line bag of per-actor helper methods built on the legacy abstraction. After the custom trait and dead actors are removed, refactor `ActorHandles` into a small typed map of `ractor::ActorRef<ActorType>` keyed by the production actor set. This makes actor lifetimes explicit and removes the last large façade between callers and the runtime.

## Acceptance Criteria

- [ ] `ActorHandles` exposes only the production actors: `config`, `provider`, `io`, `session`, `permission`, `turn`, `input`, `agent`, and `fff_indexer`.
- [ ] Each handle is a `ractor::ActorRef<Msg>` (or a thin newtype around it) rather than a custom helper struct.
- [ ] All callers in `runie-tui/src/main.rs`, `runie-cli/src/acp.rs`, `runie-agent/src/actor.rs`, and tests are updated to use the new map.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `actor_handles_contains_only_production_actors` — verifies the collapsed struct exposes exactly the expected typed actor refs and no dead fields remain.

### Layer 2 — Event Handling
- [ ] `actor_handles_send_message_to_each_actor` — sends a message through every handle in the map and confirms the actor receives it.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `smoke_actor_handles_support_full_turn` — runs a provider-replay turn using the collapsed handle map.

## Files touched

- `crates/runie-core/src/actors/handles.rs`
- `crates/runie-core/src/actors/mod.rs`
- `crates/runie-core/src/actors/leader/actor.rs`
- `crates/runie-tui/src/main.rs`
- `crates/runie-cli/src/acp.rs`
- `crates/runie-agent/src/actor.rs`
- Any tests that construct `ActorHandles`

## Notes

- This task must not change actor message protocols; it only changes how callers obtain actor references.
- If `FffIndexerHandle` cannot be expressed as a plain `ractor::ActorRef` (it has custom `search`/`try_search` methods), keep a small dedicated wrapper but move it next to the actor instead of in the global `handles.rs`.
- Rejected alternative: keeping the large helper struct for backward compatibility. It ossifies the runtime surface and makes adding or removing actors expensive.
