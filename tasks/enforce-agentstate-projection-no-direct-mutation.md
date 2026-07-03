# Enforce `AgentState` as pure projection with no direct mutation

**Status**: todo
**Milestone**: R7
**Category**: Core / State
**Priority**: P0

**Depends on**: remove-turnstate-from-appstate
**Blocks**: none

## Description

`AgentState` is documented as a read-only projection derived from `TurnState` via `From<&TurnState>`. Despite that, production code mutates it directly in `crates/runie-tui/src/ui_actor/mod.rs:552` (`self.state.agent_state_mut().turn_active = false`) and `crates/runie-core/src/update/system.rs:117` (`self.agent_state_mut().token_tracker = ...`). These direct writes create stale or inconsistent UI state.

## Acceptance Criteria

- [ ] Remove all `agent_state_mut()` accessors from production code (or make them test-only).
- [ ] Replace every direct `AgentState` mutation with an update to the authoritative `TurnState` or with a fact emitted by the owning actor.
- [ ] `AgentState` is rebuilt only through `AgentState::from(&turn_state)` or equivalent projection.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo check --workspace` passes with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `agent_state_projection_is_pure` — for any `TurnState`, `AgentState::from(&ts)` is the only way to construct an `AgentState` used by production code.
- [ ] `no_agentstate_mut_in_production` — static grep confirms `agent_state_mut()` is absent outside tests.

### Layer 2 — Event Handling
- [ ] `turn_facts_rebuild_agent_state` — applying a sequence of turn facts produces the same `AgentState` as direct mutation did.

### Layer 3 — Rendering
- [ ] N/A — no new rendering logic.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `replay_preserves_agent_state_projection` — existing replay tests pass without direct `AgentState` mutation.

### Live Tmux Testing Session
- [ ] Start a turn, abort it, and verify the TUI correctly reflects `turn_active`, queues, and token counters.

## Files touched

- `crates/runie-core/src/model/state/app_state.rs`
- `crates/runie-core/src/update/system.rs`
- `crates/runie-tui/src/ui_actor/mod.rs`
- Any test files using `agent_state_mut()`

## Notes

- Supersedes the remaining work from `treat-agentstate-as-pure-turnstate-projection.md`.
- Consider making `agent_state_mut()` `#[cfg(test)]` only, or deleting it entirely and updating tests to mutate `TurnState` instead.
