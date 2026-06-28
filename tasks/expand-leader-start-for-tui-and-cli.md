# Expand `Leader::start` for the full TUI and CLI runtime

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: collapse-actor-handles-to-typed-map
**Blocks**: migrate-tui-and-cli-to-leader-bootstrap

## Description

`crates/runie-core/src/actors/leader/actor.rs` currently spawns only a subset of actors and does not expose everything the TUI and CLI need. This task expands `Leader::start` (and `LeaderHandle`) to become the canonical shared bootstrap. Specifically:

- Spawn the full production actor set: `ConfigActor`, `ProviderActor`, `IoActor`, `SessionActor`, `RactorPermissionActor`, `RactorTurnActor`, `InputActor`, `AgentActor`, and `FffIndexerActor`.
- Return a richer `LeaderHandle` that exposes actor refs, event bus subscription, a render snapshot channel, and a graceful shutdown signal.
- Default to `embedded()` (no TCP listener) for the TUI and CLI; keep TCP listening optional.
- Do **not** migrate the TUI or CLI callers yet; that is the next task.

## Acceptance Criteria

- [ ] `Leader::start` spawns all production actors and returns a `LeaderHandle`.
- [ ] `LeaderHandle` exposes typed refs for `config`, `provider`, `io`, `session`, `permission`, `turn`, `input`, `agent`, and `fff_indexer`.
- [ ] `LeaderHandle` exposes `subscribe()` for facts, `snapshot_rx()` for the render snapshot channel, and `shutdown()` for graceful shutdown.
- [ ] The TUI and CLI still compile with their existing bootstrap; no caller migration happens in this task.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `leader_handle_exposes_all_actor_refs` — constructs a `LeaderHandle` and asserts every expected field is present and correctly typed.

### Layer 2 — Event Handling
- [ ] `leader_start_publishes_fact_on_event_bus` — starts a leader, emits a fact from one actor, and asserts subscribers receive it.
- [ ] `leader_shutdown_stops_all_actors` — calls `shutdown()` and asserts all actor refs report stopped.

### Layer 3 — Rendering
- [ ] `leader_snapshot_channel_receives_first_frame` — verifies the render snapshot channel delivers an `AppState`/`Snapshot` after startup.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `smoke_leader_runs_mock_turn` — runs a full provider-replay turn through the leader-spawned actors.

## Files touched

- `crates/runie-core/src/actors/leader/actor.rs`
- `crates/runie-core/src/actors/leader/messages.rs`
- `crates/runie-core/src/actors/leader/mod.rs`
- `crates/runie-core/src/actors/handles.rs` (if `ActorHandles` is consumed by `Leader`)
- `crates/runie-core/src/actors/mod.rs`

## Notes

- Keep the TCP listener path optional and off by default for the TUI/CLI. The long-running server mode can opt in explicitly.
- The render snapshot channel should preserve the TUI's current backpressure/drop-frame behavior; do not switch to an unbounded channel.
- Rejected alternative: migrating callers while expanding `Leader`. Doing both at once makes it impossible to tell whether a bug is in the bootstrap expansion or in caller migration.
