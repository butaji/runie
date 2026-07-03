# Remove `TurnState` from `AppState`

**Status**: todo
**Milestone**: R7
**Category**: Core / State
**Priority**: P0

**Depends on**: none
**Blocks**: enforce-agentstate-projection-no-direct-mutation

## Description

`AppState` currently stores `pub turn_state: TurnState` at `crates/runie-core/src/model/state/app_state.rs:58`, but the SSOT ADR states that `TurnState` is owned solely by `TurnActor`. Keeping a mutable copy in `AppState` creates dual-mutation paths and violates the "No mirrored state" principle. Production code mutates this copy directly in `update/agent/core_messages.rs`, `update/system.rs`, `update/dispatch.rs`, `update/session.rs`, and elsewhere.

## Acceptance Criteria

- [ ] Remove the `turn_state` field from `AppState`.
- [ ] Make all production code that currently reads `AppState.turn_state` consume `TurnActor` facts/events instead.
- [ ] Make all production code that currently mutates `turn_state` send a message to `TurnActor`.
- [ ] `AgentState` remains derivable from the projected facts (not from a mirrored `TurnState`).
- [ ] `cargo test --workspace` passes.
- [ ] `cargo check --workspace` passes with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `agent_state_derives_from_turn_facts` — `AgentState` can be rebuilt from the same events `TurnActor` emits.
- [ ] `appstate_has_no_turn_state_field` — static check or compile-time assertion that `AppState` no longer contains `TurnState`.

### Layer 2 — Event Handling
- [ ] `turnactor_emits_facts_for_lifecycle` — submit, abort, and complete turns emit the facts the UI needs.

### Layer 3 — Rendering
- [ ] N/A — no rendering change.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `headless_turn_replays_without_appstate_turn_state` — existing replay tests pass after removal.

### Live Tmux Testing Session
- [ ] Run a multi-tool turn in the TUI and verify that turn lifecycle (streaming, tool calls, completion) still renders correctly.

## Files touched

- `crates/runie-core/src/model/state/app_state.rs`
- `crates/runie-core/src/update/agent/core_messages.rs`
- `crates/runie-core/src/update/agent/core/mod.rs`
- `crates/runie-core/src/update/system.rs`
- `crates/runie-core/src/update/dispatch.rs`
- `crates/runie-core/src/update/session.rs`
- `crates/runie-core/src/model/state/turn_projections.rs`

## Notes

- Supersedes the remaining work from `remove-direct-turn-lifecycle-mutations-outside-turnactor.md` and `remove-direct-appstate-mutation-from-core-update-handlers.md`.
- This is a large refactor; consider splitting into smaller PRs by subsystem (queue, streaming, lifecycle).
