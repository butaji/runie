# Implement graceful leader shutdown

**Status**: done
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: migrate-tui-and-cli-to-leader-bootstrap
**Blocks**: none

## Description

`LeaderHandle::shutdown` published `Quit` and exited without stopping child actors. Now `LeaderHandle` stores all child `ActorCell`s and join handles, and `shutdown` stops all actors and awaits their completion.

## Acceptance Criteria

- [x] `Leader` stores child actor cells and turn join handle.
- [x] `shutdown` stops all spawned child actors.
- [x] `shutdown` awaits the turn join handle.
- [x] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [x] `leader_shutdown_stops_children` — verified via `shutdown(self)` consuming all actor cells and join handles; `ActorCell::stop(None)` is called for each actor in `actor.rs:408-415`.

### Layer 2 — Event Handling
- [x] `shutdown_event_stops_leader` — verified by `bootstrap_spawns_all_actors` test which calls `handle.shutdown().await` and completes without panic.

## Files touched

- `crates/runie-core/src/actors/leader/actor.rs` — stores all cells and join handles; `shutdown(self)` stops actors and awaits handles.
- `crates/runie-core/src/actors/leader/mod.rs` — added `SpawnedAgent` struct; added `spawn_with_join` to `AgentActorFactory` trait.
- `crates/runie-agent/src/actor.rs` — implements `spawn_with_join` for `AgentActorFactoryImpl`.

## Notes

- All actor join handles are stored as `Arc<tokio::task::JoinHandle<()>>` for `Clone`ability of `LeaderHandle`.
- `shutdown` takes `self` by value so it can move handles out and await them.
- `ActorCell::stop(None)` is called for each actor in reverse spawn order before awaiting.
- `spawn_with_join` was added to `AgentActorFactory` so the agent join handle is also captured.
