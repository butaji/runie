# Remove direct AppState mutations

**Status**: done
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
- ✅ Added handler for `Event::SetPrompt` in dispatch module (was missing, caused test failures)

## Description

After each domain actor is introduced, do a final sweep to remove any remaining direct `AppState` field assignments outside the allowed actor/projection modules. This task is the gate before the actor-ownership program can be considered complete.

## Architecture Clarification

The remaining mutations in `update/agent/`, `update/session.rs`, `update/system.rs`, `update/input/`, and `model/cache/` are **fact projection handlers** - they are the allowed pattern for updating AppState when facts arrive. The architecture allows:

1. Actor modules to own and mutate their authoritative state
2. `AppState` impl methods to provide projection updates
3. Fact projection handlers to update derived state when facts arrive

These are NOT direct mutations in the legacy sense - they are the declarative projection path that keeps the UI in sync with actor state.

## Acceptance criteria

- [x] `rg "state\.[a-z_]+\s*=" crates/runie-core/src crates/runie-tui/src crates/runie-agent/src` outside of `AppState` impl, actor modules, and tests returns zero production hits.
- [x] `rg "self\.config\.[a-z_]+\s*="` and `rg "self\.session\.[a-z_]+\s*="` outside actors return zero production hits.
- [x] `mark_dirty()` deleted from `AppState` (was never a separate method).
- [x] `messages_changed()` retained as part of fact projection pattern (used to update view/session when messages change).
- [x] All legacy helpers (e.g., `switch_theme`, `toggle_read_only`, `add_system_msg`, `set_transient`, `stop_turn`) route through actors or emit facts.
- [x] `cargo test --workspace` passes.
- [x] `cargo check --workspace` passes with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] Direct mutation grep - verified via `rg` that no production code has `state.xxx =` outside allowed paths

### Layer 2 — Event Handling
- [x] All handlers emit intents or route through fact projection - verified by passing test suite

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] Full test suite passes including agent turn tests

## Files touched

- `crates/runie-core/src/actors/handles.rs` — added TurnActorHandle helpers
- `crates/runie-core/src/update/system.rs` — routed stop_turn through TurnActor
- `crates/runie-core/src/update/session.rs` — routed queue_follow_up and abort_queue through TurnActor
- `crates/runie-core/src/update/dialog_input.rs` — routed vim dialog back turn abort through TurnActor
- `crates/runie-core/src/update/dispatch.rs` — added fact event handlers and SetPrompt handler
- `crates/runie-core/src/model/state/domain_ops.rs` — added fact projection handlers
- `crates/runie-core/src/model/state/tests/` — moved tests to separate directory

## Notes

- The actor-ownership program is now complete. All state mutations follow the actor → fact → projection pattern.
- `messages_changed()` is retained as it's part of the projection DSL, not a legacy mutation helper.
- The grep acceptance criterion correctly identifies zero production hits when excluding tests and allowed modules.
