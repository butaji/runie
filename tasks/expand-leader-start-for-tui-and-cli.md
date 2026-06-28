# Expand `Leader::start` for the full TUI and CLI runtime

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: collapse-actor-handles-to-typed-map
**Blocks**: migrate-tui-and-cli-to-leader-bootstrap

## Description

`crates/runie-core/src/actors/leader/actor.rs` currently spawns only a subset of actors and does not expose everything the TUI and CLI need. This task expands `Leader::start` (and `LeaderHandle`) to become the canonical shared bootstrap.

Current state as of Round 1:

- `Leader::start` spawns six actors: `RactorConfigActor`, `ProviderActor`, `IoActor`, `SessionActor`, `RactorPermissionActor`, and `RactorTurnActor`.
- `ProviderActor`, `IoActor`, and `SessionActor` are still legacy custom-trait actors and must be switched to their ractor versions by `migrate-production-actors-to-ractor`.
- There is a **type mismatch**: `RactorPermissionActor::spawn` returns `RactorPermissionHandle`, but `SpawnedHandles.permission` and `LeaderHandle.permission` are declared as the legacy `PermissionActorHandle`.
- `Leader::new()` defaults to TCP `127.0.0.1:9000`, which will fail if the port is in use and prevents running multiple leaders in tests.
- `Leader::start` is **not called anywhere in production**; both the TUI and CLI still bootstrap manually.
- `LeaderHandle` lacks `input`, `agent`, and `fff_indexer` refs, a render snapshot channel, and robust shutdown.

This task expands the leader after the actor migration and handle collapse are done.

## Acceptance Criteria

- [ ] `Leader::start` spawns all production actors: `RactorConfigActor`, `RactorProviderActor`, `RactorIoActor`, `RactorSessionActor`, `RactorPermissionActor`, `RactorTurnActor`, `InputActor`, `AgentActor`, and `FffIndexerActor`.
- [ ] `LeaderHandle` exposes typed refs for `config`, `provider`, `io`, `session`, `permission`, `turn`, `input`, `agent`, and `fff_indexer`.
- [ ] `LeaderHandle` exposes `subscribe()` for facts, `snapshot_rx()` for the render snapshot channel, and `shutdown()` for graceful shutdown.
- [ ] `Leader::new()` defaults to **embedded mode** (no TCP listener). `with_tcp_addr` is used for long-running server mode.
- [ ] `Leader::start` accepts `project_root` and `data_dir` (or sensible defaults) for `FffIndexerActor`.
- [ ] `AgentActor` is provided to `Leader::start` via a factory/trait to avoid a `runie-core` → `runie-agent` dependency cycle.
- [ ] Fix the `PermissionActorHandle` / `RactorPermissionHandle` type mismatch so `Leader::start` compiles.
- [ ] The TUI and CLI still compile with their existing bootstrap; no caller migration happens in this task.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `leader_handle_exposes_all_actor_refs` — constructs a `LeaderHandle` and asserts every expected field is present and correctly typed.

### Layer 2 — Event Handling
- [ ] `leader_start_publishes_fact_on_event_bus` — starts a leader, emits a fact from one actor, and asserts subscribers receive it.
- [ ] `leader_shutdown_stops_all_actors` — calls `shutdown()` and asserts all actor refs report stopped.
- [ ] `leader_default_embedded_no_tcp` — asserts that `Leader::new()` does not bind a TCP socket.

### Layer 3 — Rendering
- [ ] `leader_snapshot_channel_receives_first_frame` — verifies the render snapshot channel delivers a `Snapshot` after startup.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `smoke_leader_runs_mock_turn` — runs a full provider-replay turn through the leader-spawned actors.

## Files touched

- `crates/runie-core/src/actors/leader/actor.rs`
- `crates/runie-core/src/actors/leader/messages.rs`
- `crates/runie-core/src/actors/leader/mod.rs`
- `crates/runie-core/src/actors/handles.rs`
- `crates/runie-core/src/actors/mod.rs`
- `crates/runie-agent/src/actor.rs` (factory trait if needed)

## Notes

- Fix the permission-handle type mismatch first; otherwise `Leader::start` cannot compile even before the remaining actors are migrated.
- The TCP listener path should be optional and off by default for the TUI/CLI. The long-running server mode can opt in explicitly with `with_tcp_addr`.
- The render snapshot channel should preserve the TUI's current backpressure/drop-frame behavior; do not switch to an unbounded channel.
- `AgentActor` cannot be constructed directly in `runie-core` because it lives in `runie-agent`. Introduce a `AgentActorFactory` trait in `runie-core` and implement it in `runie-agent`, or accept a pre-spawned `ractor::ActorRef<AgentMsg>`.
- Rejected alternative: migrating callers while expanding `Leader`. Doing both at once makes it impossible to tell whether a bug is in the bootstrap expansion or in caller migration.
