# Actor-Owned State SSOT

**Status**: in_progress
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: none
**Blocks**: app-state-read-only-projection, input-actor-owns-input-state, session-actor-owns-session-state, view-actor-owns-view-state, completion-actor-owns-completion-state, turn-actor-owns-agent-turn-state, permission-actor-owns-approvals, notification-actor-owns-transient-messages, trust-actor-owns-trust-decisions, env-actor-owns-git-cwd, fff-indexer-owns-file-picker-results, remove-login-config-test-shim, ui-control-actor-owns-dialog-state, unify-approval-decision, consolidate-settings-providers-dialog, unified-dsl-intents-for-state-mutations

## Description

Define and document the actor ownership model for Runie's state. Each actor owns exactly one slice of mutable state, and no production code mutates actor-owned state outside the actor.

**Current state**: The codebase already has actors (`ConfigActor`, `SessionActor`, `ProviderActor`, `IoActor`, `FffIndexerActor`, `SessionActor`) with defined message types. This task formalizes the ownership model and fills any gaps.

## Actor State Ownership Map

| Actor | State Slice | Message Type | Notes |
|-------|-------------|--------------|-------|
| ConfigActor | `config` | `ConfigMsg` | Owns `~/.runie/config.toml` |
| SessionActor | `session` | `SessionMsg` | Owns session persistence |
| ProviderActor | `providers` | `ProviderMsg` | Owns provider credentials |
| IoActor | `io` | `IoMsg` | Owns subprocess/file IO |
| FffIndexerActor | `fff_index` | `FffMsg` | Owns file search index |
| UiControlActor | `ui_state` | (TBD) | Owns dialog state, quit, etc. |
| InputActor | `input` | (TBD) | Owns text input state |
| PermissionActor | `permissions` | (TBD) | Owns approval queue |

## Acceptance Criteria

- [x] Actor ownership map documented above
- [x] ConfigActor owns config (already implemented)
- [x] SessionActor owns session state (already implemented)
- [x] FffIndexerActor owns file picker results (already implemented)
- [ ] Missing actors documented with implementation plan:
  - [ ] UiControlActor — owns `should_quit`, `open_dialog`, `dialog_back_stack`, `login_flow`
  - [ ] InputActor — owns text input state, cursor, history
  - [ ] PermissionActor — owns `permission_request` queue
- [ ] No production code directly mutates actor-owned state outside the actor
- [ ] `cargo test --workspace` passes

## Tests

### Layer 1 — State/Logic
- N/A (documentation task)

### Layer 2 — Event Handling
- N/A (documentation task)

### Layer 3 — Rendering
- N/A

### Layer 4 — Provider Replay / Mock-Tool E2E
- N/A

## Files touched

- `tasks/actor-owned-state-ssot.md` (this file)

## Notes

- This is a foundational task that documents existing architecture and identifies gaps
- Implementation of missing actors is handled by dependent tasks
- The actor ownership model is already largely in place; this task formalizes it
