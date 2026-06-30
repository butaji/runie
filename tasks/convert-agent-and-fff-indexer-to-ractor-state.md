# Convert `AgentActor` and `FffIndexerActor` to idiomatic `ractor` `State`

**Status**: done
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: use-ractor-state-for-actor-mutable-state
**Blocks**: none

## Description

`runie-agent/src/actor.rs` holds mutable actor-local state in `Arc<Mutex<Option<...>>>` with `type State = ()`. `ractor_fff_indexer.rs` holds state in `self` fields. Move actor-local state into `type State = ...` and mutate via `&mut State`.

## Acceptance Criteria

- [x] `AgentActor` uses `type State` for provider/permission handles.
- [x] `FffIndexerActor` uses `type State` for `indexed`/`init_done`.
- [x] Remove interior `Mutex`/`Arc<Mutex>` used only for actor-local state.
- [x] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [x] `agent_actor_state_updates_without_mutex` — state updates via `&mut State`.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `mock_turn_completes_after_state_refactor` — provider replay turn works.

## Files touched

- `crates/runie-agent/src/actor.rs`
- `crates/runie-core/src/actors/fff_indexer/ractor_fff_indexer.rs`

## Notes

- `ctx7` for `ractor` confirms the `type State` pattern.
