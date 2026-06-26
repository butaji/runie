# Remove direct AppState mutations

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: session-actor-owns-session-state, input-actor-owns-input-state, view-actor-owns-view-state, completion-actor-owns-completion-state, turn-actor-owns-agent-turn-state, permission-actor-owns-approvals, notification-actor-owns-transient-messages, trust-actor-owns-trust-decisions, env-actor-owns-git-cwd, fff-indexer-owns-file-picker-results, ui-control-actor-owns-dialog-state, config-ssot-via-configactor, actor-lifecycle-and-handle-registry
**Blocks**: none

## Progress

**Completed changes:**
- ✅ `stop_turn()` now routes through `TurnActor::AbortTurn` 
- ✅ `abort_queue()` now routes through `TurnActor::AbortQueue`
- ✅ `queue_follow_up()` now routes through `TurnActor::QueueFollowUp`
- ✅ Added fact projection handlers: `apply_turn_aborted()`, `apply_turn_completed()`, `apply_turn_errored()`, `apply_token_stats()`
- ✅ Added fact projection handlers: `apply_turn_started()` for TurnStarted events
- ✅ `handle_vim_dialog_back()` now routes turn abort through TurnActor
- ✅ Added TurnActorHandle helpers for all queue and lifecycle operations
- ✅ Added fact event handlers in dispatch module for TurnAborted, QueueAborted, TurnStarted, TurnCompleted, TurnErrored, TokenStatsUpdated

**Remaining work:**
- `update/agent/core/mod.rs` - `set_thinking`, `add_thought`, `start_tool`, `end_tool`, `append_response*` still mutate state directly
- `update/agent/core_messages.rs` - AgentCoreMessage handlers still mutate state
- `update/session.rs` - `push_user_message`, `deliver_queued`, `dequeue`, `try_deliver_*` still access queue state directly
- `update/system.rs` - `peek_queue`, `pop_queue`, `configure_token_tracker` still access state directly
- `update/input/submit.rs` - `submit_user_message` and related still mutate state
- `update/input/mod.rs` - `handle_escape` checks `turn_active` directly
- `model/cache/mod.rs` - speed/tokens animation still updates state directly

## Description

After each domain actor is introduced, do a final sweep to remove any remaining direct `AppState` field assignments outside the allowed actor/projection modules. This task is the gate before the actor-ownership program can be considered complete.

## Acceptance criteria

- [ ] `rg "state\.[a-z_]+\s*=" crates/runie-core/src crates/runie-tui/src crates/runie-agent/src` outside of `AppState` impl, actor modules, and tests returns zero production hits.
- [ ] `rg "self\.config\.[a-z_]+\s*="` and `rg "self\.session\.[a-z_]+\s*="` outside actors return zero production hits.
- [ ] `mark_dirty()` and `messages_changed()` helpers are deleted from `AppState`.
- [ ] All legacy helpers that mutated state (e.g., `switch_theme`, `toggle_read_only`, `add_system_msg`, `set_transient`, `apply_trust_project`, `stop_turn`) are either deleted or converted to intent emitters.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo check --workspace` passes with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `no_direct_mutation_grep` — script/grep test that fails the build if direct mutations reappear.

### Layer 2 — Event Handling
- [ ] `all_handlers_emit_intents` — property-style test that feeds synthetic events and asserts no direct field writes.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `full_smoke_no_direct_mutations` — run a mock-provider E2E turn and assert the direct-mutation guard never trips.

## Files touched

- `crates/runie-core/src/actors/handles.rs` — added TurnActorHandle helpers
- `crates/runie-core/src/update/system.rs` — routed stop_turn through TurnActor
- `crates/runie-core/src/update/session.rs` — routed queue_follow_up and abort_queue through TurnActor
- `crates/runie-core/src/update/dialog_input.rs` — routed vim dialog back turn abort through TurnActor
- `crates/runie-core/src/update/dispatch.rs` — added fact event handlers
- `crates/runie-core/src/model/state/domain_ops.rs` — added fact projection handlers
- `crates/runie-core/src/model/state/tests/` — moved tests to separate directory

## Notes

- This task must run last in the actor-ownership program.
- The grep acceptance criteria can be enforced by a small shell test in `scripts/verify-tests.sh` or a build-script check.
- Remaining work requires TurnActor to expose queue query methods and agent event routing.
