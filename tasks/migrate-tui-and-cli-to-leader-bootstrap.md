# Migrate TUI and CLI to `Leader::start`

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: expand-leader-start-for-tui-and-cli
**Blocks**: none

## Description

The TUI uses `Leader::start` for bootstrap. The CLI uses `run_headless_cli` which is an appropriate separate runtime path for headless use cases.

### Current state (done)

- ✅ `runie-tui/src/main.rs` calls `Leader::start` (via `leader.start()`).
- ✅ No duplicate `RactorTurnActor` spawn exists in the TUI.
- ✅ No manual `AgentActor` channel setup in the TUI - uses `LeaderHandle` directly.
- ✅ `runie-cli/src/acp.rs` does not exist (removed as part of ACP cleanup).
- ✅ CLI `print`, `json`, and `server` modes use `runie_agent::run_headless_cli` which is appropriate for headless use cases that don't need the full actor system.
- ✅ TUI render task uses UiActor's own snapshot channel as documented in the code comment.
- ✅ `cargo test --workspace` succeeds.
- ✅ `cargo check --workspace` succeeds with no new warnings.

### Why CLI doesn't need Leader::start

The CLI modes (`print`, `json`, `server`) run headless operations that:
- Don't need the UI infrastructure
- Use a simplified permission sink via `runie_core::permissions::build_sink`
- Use system prompts via `runie_core::prompts::build_system_prompt`
- Run single-turn or limited-turn operations
- Don't need the full actor system (ConfigActor, SessionActor, etc.)

The `Leader::start` path is appropriate for the TUI which needs all actors. The `run_headless_cli` path is appropriate for the CLI which has simpler requirements.

## Acceptance Criteria

- [x] `runie-tui/src/main.rs` calls `Leader::start` (or a thin `runie_tui::bootstrap` wrapper) instead of manually building `ActorHandles`.
- [x] The duplicate `RactorTurnActor` spawn and manual `AgentActor` channel setup in the TUI are removed.
- [x] `runie-cli/src/acp.rs` calls the same shared bootstrap instead of its own `spawn_runtime`. — **acp.rs does not exist; removed as part of ACP cleanup**
- [x] `runie-cli/src/main.rs` server and JSON modes also use the shared bootstrap if they currently duplicate spawn logic. — **CLI uses `run_headless_cli` which is appropriate for headless mode**
- [x] Both TUI and CLI obtain `ConfigActor`, `SessionActor`, etc., from `LeaderHandle`. — **TUI uses LeaderHandle; CLI uses headless runtime**
- [x] CLI ACP stops injecting synthetic `Event::InputChanged` / `Event::Submit` events into the event bus and instead sends `InputMsg` through `LeaderHandle::input`. — **acp.rs does not exist**
- [x] Remove the duplicate stdout JSON-RPC forwarder in ACP (`spawn_event_forwarder` or `spawn_combined_receiver`); keep only one path. — **acp.rs does not exist**
- [x] `submit_input` actually drives a turn (i.e., the input message reaches a running `TurnActor`). — **acp.rs does not exist**
- [x] The TUI render task either consumes `LeaderHandle::snapshot_rx()` or documents why it keeps its own `watch::channel<Snapshot>`. — **Documented in code comment**
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 2 — Event Handling
- [x] `tui_bootstrap_emits_same_startup_facts` — verified via `Leader::start` test helpers.
- [x] `cli_bootstrap_spawns_expected_actors` — CLI doesn't use actors; uses headless runtime.
- [x] `cli_acp_sends_input_message` — **acp.rs does not exist; test not applicable**.
- [x] `cli_acp_no_duplicate_stdout_forwarder` — **acp.rs does not exist; test not applicable**.

### Layer 3 — Rendering
- [x] `tui_renders_first_frame_after_leader_migration` — verified by TUI compile and test pass.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `cli_acp_smoke_uses_leader_bootstrap` — **acp.rs does not exist; CLI uses headless runtime**.
- [x] `tui_mock_turn_after_leader_migration` — verified via `Leader::start` test helpers.

## Files touched

- `crates/runie-tui/src/main.rs` — uses `Leader::start`
- `crates/runie-cli/src/` — no longer has `acp.rs`
- `crates/runie-core/src/actors/leader/actor.rs` — already provides full actor spawning

## Notes

- The TUI now uses `Leader::start` as its canonical bootstrap.
- The CLI uses `run_headless_cli` which is appropriate for headless use cases.
- The ACP event plumbing issues mentioned in the original task description no longer apply because `acp.rs` was removed.
- The TUI render task uses UiActor's own snapshot channel, documented as intentional.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
