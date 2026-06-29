# Reconnect or remove TUI `AgentActor` channel

**Status**: todo
**Milestone**: R6
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: migrate-production-actors-to-ractor
**Blocks**: migrate-tui-and-cli-to-leader-bootstrap

## Description

`crates/runie-tui/src/main.rs` creates an `mpsc::channel<AgentMsg>`, drops the receiver, and discards the real `RactorAgentHandle` returned by `spawn_ractor_agent`. `UiActor` sends `AgentMsg::Run` into a channel with no consumer. The TUI therefore cannot drive agent turns in production.

## Acceptance Criteria

- [ ] Either forward `AgentMsg` from the mpsc receiver to the real `RactorAgentHandle`, or replace `AgentActorHandle` with `RactorAgentHandle`.
- [ ] The TUI can submit a turn and receive `TurnComplete`/`ToolCall*` events.
- [ ] Remove the disconnected `agent_tx` from `UiActor` if it is no longer needed.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 2 — Event Handling
- [ ] `tui_agent_run_reaches_ractor_actor` — sending `AgentMsg::Run` from the TUI results in a message to the real actor.
- [ ] `tui_agent_handle_is_real_ractor_handle` — `AgentActorHandle` wraps `RactorAgentHandle`.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tui_turn_completes_via_real_agent` — a TUI-driven provider replay turn completes end-to-end.

## Files touched

- `crates/runie-tui/src/main.rs`
- `crates/runie-tui/src/ui_actor.rs`
- `crates/runie-agent/src/actor.rs`

## Notes

- This is a prerequisite for `migrate-tui-and-cli-to-leader-bootstrap.md`.
- If `Leader::start` is used, the TUI should obtain the agent handle from the leader instead of spawning manually.
