# Event Taxonomy for Actor State Sync

**Status**: in_progress
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: none
**Blocks**: app-state-read-only-projection, input-actor-owns-input-state, session-actor-owns-session-state, view-actor-owns-view-state, completion-actor-owns-completion-state, turn-actor-owns-agent-turn-state, permission-actor-owns-approvals, notification-actor-owns-transient-messages, trust-actor-owns-trust-decisions, env-actor-owns-git-cwd, fff-indexer-owns-file-picker-results

## Description

Document and enforce the event taxonomy for state synchronization. Events are classified into three categories: **Intents** (requests), **Facts** (state changes), and **Control** (app lifecycle).

**Current state**: The codebase already implements this taxonomy in `crates/runie-core/src/event/`. This task verifies the implementation and fills any gaps.

## Event Taxonomy

### Intents (requests to actors)
- Fire-and-forget requests
- Handlers emit intents → actors receive and process
- Examples: `Intent::SetTheme`, `Intent::Submit`, `Intent::TrustProject`

### Facts (state changes from actors)
- Produced by actors after processing intents
- Projected into `AppState` by the UI layer
- Examples: `Fact::ConfigLoaded`, `Fact::SessionChanged`, `Fact::TurnProgress`

### Control (app lifecycle)
- System-level events
- Handled by the main event loop
- Examples: `Control::Quit`, `Control::Suspend`

## Current Implementation Status

| Category | Location | Status |
|----------|---------|--------|
| Intent enum | `event/intent.rs` | ✅ Implemented |
| Fact enum | `event/fact.rs` | ✅ Implemented |
| Control enum | `event/control.rs` | ✅ Implemented |
| EventKind classification | `event/kind.rs` | ✅ Implemented |
| Intent → Actor routing | `event/routing.rs` | ✅ Implemented |

## Acceptance Criteria

- [x] Event taxonomy documented
- [x] `Intent` enum covers all actor requests
- [x] `Fact` enum covers all state changes
- [x] `Control` enum covers app lifecycle
- [x] `EventKind` classifies each event correctly
- [x] Intent → Actor routing documented
- [ ] Missing fact types documented:
  - [ ] `Fact::InputStateChanged` — for InputActor state changes
  - [ ] `Fact::PermissionRequested` — for PermissionActor queue
  - [ ] `Fact::DialogStateChanged` — for UiControlActor state
- [ ] `cargo test --workspace` passes

## Tests

### Layer 1 — State/Logic
- [ ] `event_kind_classifies_intents_correctly`
- [ ] `event_kind_classifies_facts_correctly`
- [ ] `event_kind_classifies_control_correctly`

### Layer 2 — Event Handling
- N/A (documentation task)

### Layer 3 — Rendering
- N/A

### Layer 4 — Provider Replay / Mock-Tool E2E
- N/A

## Files touched

- `tasks/event-taxonomy-for-actor-state-sync.md` (this file)
- `crates/runie-core/src/event/intent.rs` (verify completeness)
- `crates/runie-core/src/event/fact.rs` (verify completeness)
- `crates/runie-core/src/event/control.rs` (verify completeness)

## Notes

- This is a foundational task that documents existing architecture
- Missing fact types should be implemented in dependent tasks
- The event taxonomy is already largely in place; this task formalizes it
