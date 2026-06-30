# Expand `Leader::start` for the full TUI and CLI runtime

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: collapse-actor-handles-to-typed-map
**Blocks**: migrate-tui-and-cli-to-leader-bootstrap

## Description

`Leader::start` now spawns all 9 production actors and `LeaderHandle` exposes all typed refs. The TUI uses `Leader::start` as its canonical bootstrap.

## Current state (done)

- ✅ `Leader::start` spawns all production actors: `RactorConfigActor`, `RactorProviderActor`, `RactorIoActor`, `RactorSessionActor`, `RactorPermissionActor`, `RactorTurnActor`, `InputActor`, `AgentActor` (via factory), and `FffIndexerActor`.
- ✅ `LeaderHandle` exposes typed refs for `config`, `provider`, `io`, `session`, `permission`, `turn`, `input`, `agent`, and `fff_indexer`.
- ✅ `LeaderHandle` exposes `subscribe()` for facts and `shutdown()` for graceful shutdown. The render snapshot channel is managed by `UiActor::take_render_rx()`.
- ✅ `Leader::new()` defaults to **embedded mode** (no TCP listener). `with_tcp_addr` is used for long-running server mode.
- ✅ `Leader::start` accepts `project_root` and `data_dir` via `LeaderConfig` for `FffIndexerActor`.
- ✅ `AgentActor` is provided to `Leader::start` via `AgentActorFactory` trait to avoid a `runie-core` → `runie-agent` dependency cycle.
- ✅ `LeaderHandle` has a `snapshot_rx` field (`watch::Receiver<Snapshot>`) for snapshot delivery verification.
- ✅ The TUI uses `Leader::start` directly.

## Acceptance Criteria

- [x] `Leader::start` spawns all production actors.
- [x] `LeaderHandle` exposes typed refs for all actors.
- [x] `LeaderHandle` exposes `subscribe()` for facts and `shutdown()` for graceful shutdown.
- [x] `Leader::new()` defaults to embedded mode.
- [x] `Leader::start` accepts `project_root` and `data_dir` for `FffIndexerActor`.
- [x] `AgentActor` is provided via factory/trait.
- [x] TUI uses `Leader::start`.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `leader_handle_has_all_actor_fields` — verifies all expected typed `ActorRef` fields on `LeaderHandle`.
- [x] `actor_handles_is_leader_handle` — `ActorHandles` alias resolves to `LeaderHandle`.

### Layer 2 — Event Handling
- [x] `leader_config_send_reaches_actor` — config message sent via `LeaderHandle.config`.
- [x] `leader_session_send_reaches_actor` — session message via `LeaderHandle.session`.
- [x] `leader_turn_send_reaches_actor` — turn message via `LeaderHandle.turn`.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `bootstrap_spawns_all_actors` — `Leader::start()` produces a `LeaderHandle` with all fields.

## Files touched

- `crates/runie-core/src/actors/leader/actor.rs` — all actors spawned
- `crates/runie-core/src/actors/leader/mod.rs` — factory traits and `LeaderHandle` re-exports
- `crates/runie-core/src/actors/handles.rs` — `ActorHandles` alias
- `crates/runie-tui/src/main.rs` — uses `Leader::start`

## Notes

- The TUI now uses `Leader::start` as its canonical bootstrap.
- The render snapshot channel is owned by `UiActor` via `UiActor::take_render_rx()`; `LeaderHandle::snapshot_rx` is a placeholder for verification.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
