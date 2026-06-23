# AppState as read-only projection

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P0

**Depends on**: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync
**Blocks**: session-actor-owns-session-state, input-actor-owns-input-state, view-actor-owns-view-state, completion-actor-owns-completion-state, turn-actor-owns-agent-turn-state, permission-actor-owns-approvals, notification-actor-owns-transient-messages, trust-actor-owns-trust-decisions, env-actor-owns-git-cwd, fff-indexer-owns-file-picker-results

## Description

`AppState` is currently a bag of public mutable fields that anyone can write. Convert it into an immutable projection of actor-owned facts. Only the `update(event)` dispatcher and actor internals may change state; actors change the authoritative copy they hold.

## Acceptance criteria

- [ ] All domain fields on `AppState` (`session`, `input`, `view`, `completion`, `agent` turn subset, `config`, `trust_decisions`, `transient_*`, `fff_*`, `git_info`, `cwd_name`, `permission_request`) are made private.
- [ ] `AppState` exposes immutable accessors for each domain slice.
- [ ] `AppState::update(event: Event)` is the only non-actor production code path that mutates these fields.
- [ ] `AppState` exposes thin `send_*_msg` helpers for actor handles; no logic lives there.
- [ ] `reset_session` preserves actor handles, trust, env, and approval registry references.
- [ ] A debug-only compile-time or runtime guard rejects direct mutation attempts outside the allowed modules.
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [ ] `app_state_update_applies_config_loaded` — `ConfigLoaded` updates config projection.
- [ ] `app_state_update_applies_session_changed` — `SessionChanged` updates session projection.

### Layer 2 — Event Handling
- [ ] `direct_field_write_fails_to_compile` — add a `trybuild`-style test (or at least a grep check) that proves fields are private.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/model/state/app_state.rs` — make fields private, add accessors, add `update` dispatcher.
- `crates/runie-core/src/update/mod.rs` — dispatch facts to projection helpers.
- All files reading `AppState` fields — switch to accessors.
- All files writing `AppState` fields — move to actor intents.

## Notes

- This task must land after the event taxonomy is defined but can be done incrementally per actor.
- `should_quit`, `open_dialog`, and `dialog_back_stack` may need special handling; they are UI-control state. Prefer moving them to `ViewActor` if possible, or keep them under strict access control.
