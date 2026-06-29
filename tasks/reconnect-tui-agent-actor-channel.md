# Reconnect or remove TUI `AgentActor` channel

**Status**: done
**Milestone**: R6
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: migrate-production-actors-to-ractor
**Blocks**: migrate-tui-and-cli-to-leader-bootstrap

## Description

The TUI agent channel was disconnected: `main.rs` created an mpsc channel, dropped the receiver, and `UiActor` sent to a channel with no consumer. This was fixed when the TUI migrated to `Leader::start()`.

## What changed

`Leader::start()` spawns the `AgentActor` via `AgentActorFactory::spawn()` and returns its `LeaderAgentHandle`. The TUI bootstrap now:

1. Calls `Leader::start(provider_factory, agent_factory)` → gets `LeaderHandle`
2. Extracts `leader_handle.agent` (the real `LeaderAgentHandle`)
3. Wraps it as `LeaderAgentActorHandle::new(leader_handle.agent.clone())`
4. Passes to `UiActor::with_agent_handle(..., AgentHandleBox::Leader(agent_handle), ...)`

The `LeaderAgentActorHandle::run()` calls `self.inner.run(cmd)` which invokes the real `LeaderAgentHandle::run()`, delivering the command to the spawned actor. The broken mpsc channel is gone from production code (mpsc `AgentActorHandle` is only used in unit tests).

## Acceptance Criteria

- [x] Either forward `AgentMsg` from the mpsc receiver to the real `RactorAgentHandle`, or replace `AgentActorHandle` with `RactorAgentHandle`.
- [x] The TUI can submit a turn and receive `TurnComplete`/`ToolCall*` events.
- [x] Remove the disconnected `agent_tx` from `UiActor` if it is no longer needed.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 2 — Event Handling
- [x] `tui_agent_run_reaches_ractor_actor` — `LeaderAgentActorHandle::run` delegates to the real `LeaderAgentHandle`.
- [x] `tui_agent_handle_is_real_ractor_handle` — `LeaderAgentActorHandle` wraps `Arc<dyn LeaderAgentHandle>`.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] Covered by `bootstrap_spawns_all_actors` which verifies agent field is present on `LeaderHandle`.

## Files touched

- `crates/runie-tui/src/main.rs` — uses `LeaderAgentActorHandle` from `leader_handle.agent`
- `crates/runie-tui/src/ui_actor.rs` — `LeaderAgentActorHandle` wires to real actor

## Notes

- The mpsc `AgentActorHandle` is retained only for unit tests where actors are not spawned.
