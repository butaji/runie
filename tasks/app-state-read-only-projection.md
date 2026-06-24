# AppState as read-only projection

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P0

**Depends on**: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync
**Blocks**: session-actor-owns-session-state, input-actor-owns-input-state, view-actor-owns-view-state, completion-actor-owns-completion-state, turn-actor-owns-agent-turn-state, permission-actor-owns-approvals, notification-actor-owns-transient-messages, trust-actor-owns-trust-decisions, env-actor-owns-git-cwd, fff-indexer-owns-file-picker-results

## Description

`AppState` is currently a bag of public mutable fields that anyone can write. Every field in `AppState` and every field in the inner state structs (`SessionState`, `InputState`, `ViewState`, `CompletionState`, `AgentState`, `ConfigState`) is `pub`. Today `AppState::update()` exists but is **not** the sole production mutation path; `update/`, `commands/dsl/handlers/`, `login_flow/`, `notification.rs`, and `runie-tui/src/app_init.rs` mutate fields directly. `UiActor` also currently owns and mutates `AppState`.

Convert `AppState` into an immutable projection of actor-owned facts. Only the `update(event)` dispatcher may mutate the projection; actors hold authoritative state elsewhere.

## Acceptance criteria

- [ ] All domain fields on `AppState` (`session`, `input`, `view`, `completion`, `agent`, `config`, `trust_decisions`, `transient_*`, `fff_*`, `git_info`, `cwd_name`, `permission_request`, `open_dialog`, `dialog_back_stack`, `login_flow`, `should_quit`) are made private.
- [ ] Inner state structs (`SessionState`, `InputState`, `ViewState`, `CompletionState`, `AgentState`, `ConfigState`) are also encapsulated so callers cannot mutate through `state.session().messages.push(...)`.
- [ ] `AppState` exposes immutable accessors for each domain slice.
- [ ] `AppState::update(event: Fact)` is the only production code path that mutates the projection; intents are routed to actors, not applied here.
- [ ] `UiActor` stops mutating `AppState` directly; it only forwards facts to `AppState::update`.
- [ ] `reset_session` preserves actor handles, trust, env, and approval registry references.
- [ ] A staged enforcement is used: first a `grep`/`clippy` lint that fails CI on direct field writes, then a `trybuild` compile-time test once violations are gone.
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

- This task must land after the event taxonomy is defined but should be done incrementally per actor; do not attempt to privatize everything in one giant PR.
- Because hundreds of direct field writes exist today, the staged enforcement is critical: lint first, fix incrementally, then `trybuild`.
- `should_quit`, `open_dialog`, `dialog_back_stack`, and `login_flow` are UI-control state owned by `UiControlActor`; `permission_request` is owned by `PermissionActor`.
