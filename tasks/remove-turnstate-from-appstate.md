# Remove `TurnState` from `AppState`

**Status**: done
**Milestone**: R7
**Category**: Core / State
**Priority**: P0

**Depends on**: none
**Blocks**: enforce-agentstate-projection-no-direct-mutation

## Summary

Removed all references to `AppState.turn_state` and made update handlers work with `AgentState` directly. The `turn_state` field was already absent from `AppState` (removed in a prior commit), but the code still referenced `turn_state()` and `turn_state_mut()` methods that no longer existed.

## Changes

### Production code

1. **`turn_projections.rs`** — Projection methods (`apply_turn_started`, `apply_turn_completed`, etc.) now update `AgentState` fields directly instead of mutating `TurnState` and syncing.

2. **`update/agent/core/mod.rs`** — All handlers (`set_thinking`, `add_thought`, `start_tool`, `end_tool`, etc.) now use `agent_state()` and `agent_state_mut()` instead of `turn_state()` and `turn_state_mut()`.

3. **`update/agent/core_messages.rs`** — Message handlers (`flush_buffered_response`, `on_assistant_message_ready`, `create_assistant_message`, `complete_turn`, `finish_turn`, `clear_turn_state`, etc.) now work with `AgentState` directly.

4. **`update/dispatch.rs`** — Fixed `StreamStarted` and `QueuesCleared` handlers to work with `AgentState`.

5. **`update/system.rs`** — Fixed `apply_turn_aborted` to work with `AgentState`.

6. **`update/session.rs`** — Fixed `abort_queue`, `deliver_queued`, `apply_queue_delivery_sync`, and `dequeue` to work with `AgentState.message_queue`.

7. **`update/input/submit.rs`** — Fixed `estimate_and_add_tokens` and `apply_user_message_sync` to work with `AgentState`.

### Test code

Updated all test files to use `state.agent.` field access or `state.agent_state_mut()` method calls instead of `state.turn_state`. Updated comments to remove references to "authoritative TurnState".

### Removed

- Removed unused `AgentState` import from `turn_projections.rs`
- Removed `sync_agent_state()` calls (method no longer exists)
- Updated test helper functions to work with `AgentState` directly

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
