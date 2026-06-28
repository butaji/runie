# Migrate TUI and CLI to `Leader::start`

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: expand-leader-start-for-tui-and-cli
**Blocks**: route-cli-config-through-configactor

## Description

Once `Leader::start` provides the full runtime, replace the manual bootstrap in `runie-tui/src/main.rs` and `runie-cli/src/acp.rs` with a call to `Leader::start`. This removes the duplicated actor lifetimes, the second `RactorTurnActor` spawn in the TUI, and the separate CLI `spawn_runtime`.

## Acceptance Criteria

- [ ] `runie-tui/src/main.rs` calls `Leader::start` (or a thin `runie_tui::bootstrap` wrapper) instead of manually building `ActorHandles`.
- [ ] The duplicate `RactorTurnActor` spawn and manual `AgentActor` channel setup in the TUI are removed.
- [ ] `runie-cli/src/acp.rs` calls the same shared bootstrap instead of its own `spawn_runtime`.
- [ ] Both TUI and CLI obtain `ConfigActor`, `SessionActor`, etc., from `LeaderHandle`.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 2 — Event Handling
- [ ] `tui_bootstrap_emits_same_startup_facts` — compares the sequence of startup facts before and after migration.
- [ ] `cli_bootstrap_spawns_expected_actors` — reflects over the `LeaderHandle` from the CLI path and confirms the expected actor set.

### Layer 3 — Rendering
- [ ] `tui_renders_first_frame_after_leader_migration` — starts the TUI with the new bootstrap and asserts the first `Snapshot` renders without panic.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `cli_acp_smoke_uses_leader_bootstrap` — runs the ACP stdio path with the shared bootstrap and confirms fact streaming works.
- [ ] `tui_mock_turn_after_leader_migration` — runs a provider-replay turn end-to-end through the TUI bootstrap.

## Files touched

- `crates/runie-tui/src/main.rs`
- `crates/runie-tui/src/ui_actor.rs` (if it constructs handles)
- `crates/runie-cli/src/acp.rs`
- `crates/runie-cli/src/main.rs` (if spawn logic is shared)
- `crates/runie-core/src/actors/leader/actor.rs` (minor adjustments if needed)

## Notes

- This task should be mostly deletion: remove manual spawn code and replace with `Leader::start`.
- The TUI render task currently uses `watch::channel<Snapshot>` plus a dedicated OS thread; this should be replaced by the snapshot channel provided by `LeaderHandle`.
- Rejected alternative: incrementally moving callers. The duplication is small enough that a single migration commit is cleaner, and the previous task already proved `Leader::start` works in isolation.
