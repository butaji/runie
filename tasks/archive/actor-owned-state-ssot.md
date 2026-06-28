# Actor-Owned State SSOT

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: none
**Blocks**: app-state-read-only-projection, input-actor-owns-input-state, session-actor-owns-session-state, view-actor-owns-view-state, completion-actor-owns-completion-state, turn-actor-owns-agent-turn-state, permission-actor-owns-approvals, notification-actor-owns-transient-messages, trust-actor-owns-trust-decisions, env-actor-owns-git-cwd, fff-indexer-owns-file-picker-results, remove-login-config-test-shim, ui-control-actor-owns-dialog-state, unify-approval-decision, consolidate-settings-providers-dialog, unified-dsl-intents-for-state-mutations

## Description

Define and document the actor ownership model for Runie's state. Each actor owns exactly one slice of mutable state, and no production code mutates actor-owned state outside the actor.

## Actor State Ownership Map

| Actor | State Slice | Message Type | Status |
|-------|-------------|--------------|--------|
| ConfigActor | `config` | `ConfigMsg` | ✅ Implemented |
| SessionActor | `session` | `SessionMsg` | ✅ Implemented |
| ProviderActor | `providers` | `ProviderMsg` | ✅ Implemented |
| IoActor | `io` | `IoMsg` | ✅ Implemented |
| FffIndexerActor | `fff_index` | `FffSearchRequest` | ✅ Implemented |
| PermissionActor | `permissions` | `PermissionMsg` | ✅ Implemented |
| UiControlActor | `ui_state` | (planned) | 📋 Planned: owns `should_quit`, `open_dialog`, `dialog_back_stack`, `login_flow` |
| InputActor | `input` | (planned) | 📋 Planned: owns text input state, cursor, history |
| ViewActor | `view` | (planned) | 📋 Planned: owns view cache, dirty flag, scroll, animation |
| TurnActor | `turn` | (planned) | 📋 Planned: owns turn lifecycle, queues, token accounting |

## Acceptance Criteria

- [x] Actor ownership map documented above
- [x] ConfigActor owns config (implemented)
- [x] SessionActor owns session state (implemented)
- [x] FffIndexerActor owns file picker results (implemented)
- [x] Missing actors documented with implementation plan:
  - [x] UiControlActor — owns `should_quit`, `open_dialog`, `dialog_back_stack`, `login_flow`
  - [x] InputActor — owns text input state, cursor, history
  - [x] ViewActor — owns view cache, dirty flag, scroll, animation
  - [x] TurnActor — owns turn lifecycle, queues, token accounting
- [x] No production code directly mutates actor-owned state outside the actor (verified via code review)
- [x] `cargo test --workspace` passes

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

- This task documents the existing architecture and identifies planned actors
- Implementation of planned actors (ViewActor, TurnActor, InputActor, UiControlActor) is handled by dependent tasks
- The current actor ownership model uses AppState as a read-only projection of actor state
- Future refactoring will extract ViewActor, TurnActor, InputActor, and UiControlActor as dedicated actors
