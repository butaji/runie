# Migrate TUI and CLI to `Leader::start`

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: expand-leader-start-for-tui-and-cli
**Blocks**: route-cli-config-through-configactor

## Description

Once `Leader::start` provides the full runtime, replace the manual bootstrap in `runie-tui/src/main.rs` and `runie-cli` with a call to `Leader::start`. This removes the duplicated actor lifetimes, the second `RactorTurnActor` spawn in the TUI, and the separate CLI `spawn_runtime`.

Current state as of Round 1:

- `runie-tui/src/main.rs` uses `bootstrap_app()` (lines 79–115) to manually spawn actors and build `ActorHandles`.
- `runie-tui/src/main.rs` also has a duplicate `RactorTurnActor` spawn in `setup_actor_channels` (around line 177) and manually spawns `AgentActor` (around line 172).
- `runie-cli/src/acp.rs` uses `spawn_runtime()` (lines 142–163) to manually spawn actors.
- `runie-cli/src/main.rs` runs `inspect` and `mcp` synchronously; only `json`, `server`, and `acp` enter an async runtime.
- `Leader::start` is not yet called by any production code.
- **ACP event plumbing is broken:** `AcpRuntime` stores an `event_tx` but drops the receiver inside `spawn_runtime`; `submit_input` sends synthetic `Event::InputChanged` / `Event::Submit` to a channel with no consumer. Additionally, both `spawn_event_forwarder` and `spawn_combined_receiver` write JSON-RPC notifications to stdout, causing duplicate output.

## Acceptance Criteria

- [ ] `runie-tui/src/main.rs` calls `Leader::start` (or a thin `runie_tui::bootstrap` wrapper) instead of manually building `ActorHandles`.
- [ ] The duplicate `RactorTurnActor` spawn and manual `AgentActor` channel setup in the TUI are removed.
- [ ] `runie-cli/src/acp.rs` calls the same shared bootstrap instead of its own `spawn_runtime`.
- [ ] `runie-cli/src/main.rs` server and JSON modes also use the shared bootstrap if they currently duplicate spawn logic.
- [ ] Both TUI and CLI obtain `ConfigActor`, `SessionActor`, etc., from `LeaderHandle`.
- [ ] CLI ACP stops injecting synthetic `Event::InputChanged` / `Event::Submit` events into the event bus and instead sends `InputMsg` through `LeaderHandle::input`.
- [ ] Remove the duplicate stdout JSON-RPC forwarder in ACP (`spawn_event_forwarder` or `spawn_combined_receiver`); keep only one path.
- [ ] `submit_input` actually drives a turn (i.e., the input message reaches a running `TurnActor`).
- [ ] The TUI render task either consumes `LeaderHandle::snapshot_rx()` or documents why it keeps its own `watch::channel<Snapshot>`.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 2 — Event Handling
- [ ] `tui_bootstrap_emits_same_startup_facts` — compares the sequence of startup facts before and after migration.
- [ ] `cli_bootstrap_spawns_expected_actors` — reflects over the `LeaderHandle` from the CLI path and confirms the expected actor set.
- [ ] `cli_acp_sends_input_message` — verifies the ACP `submit_input` path sends an `InputMsg` instead of a synthetic `Event`.
- [ ] `cli_acp_no_duplicate_stdout_forwarder` — asserts that only one component writes JSON-RPC notifications to stdout.

### Layer 3 — Rendering
- [ ] `tui_renders_first_frame_after_leader_migration` — starts the TUI with the new bootstrap and asserts the first `Snapshot` renders without panic.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `cli_acp_smoke_uses_leader_bootstrap` — runs the ACP stdio path with the shared bootstrap and confirms fact streaming works.
- [ ] `tui_mock_turn_after_leader_migration` — runs a provider-replay turn end-to-end through the TUI bootstrap.

## Files touched

- `crates/runie-tui/src/main.rs`
- `crates/runie-tui/src/ui_actor.rs` (if it constructs handles)
- `crates/runie-cli/src/acp.rs`
- `crates/runie-cli/src/main.rs`
- `crates/runie-core/src/actors/leader/actor.rs` (minor adjustments if needed)

## Notes

- This task should be mostly deletion: remove manual spawn code and replace with `Leader::start`.
- Fixing the ACP event plumbing is a prerequisite for the ACP smoke tests; without it `submit_input` is a no-op.
- The TUI render task currently uses `watch::channel<Snapshot>` plus a dedicated OS thread; this should be replaced by the snapshot channel provided by `LeaderHandle`.
- Rejected alternative: incrementally moving callers. The duplication is small enough that a single migration commit is cleaner, and the previous task already proved `Leader::start` works in isolation.
