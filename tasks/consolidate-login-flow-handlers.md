# Consolidate Login Flow Handlers

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: event-taxonomy-for-actor-state-sync
**Blocks**: consolidate-config-modules-into-dir

## Description

Consolidate login/authentication flow handlers from scattered locations into a unified `login_flow/` module. Currently login logic is split between `login_flow/`, `update/dialog/login_*`, `commands/dsl/handlers/login_*`, etc.

## Acceptance Criteria

- [ ] All login/auth flow handlers in `login_flow/`
- [ ] Single entry point for login flow
- [ ] Provider selection consolidated
- [ ] `cargo test --workspace` passes

## Tests

### Layer 1 — State/Logic
- [ ] `login_flow_state_transitions_correctly`

### Layer 2 — Event Handling
- [ ] `login_start_intent_transitions_to_provider_selection`
- [ ] `login_cancel_returns_to_previous_state`

### Layer 3 — Rendering
- [ ] `login_dialog_renders_correctly`

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A

## Files touched

- `crates/runie-core/src/login_flow/` (new/modified)
- `crates/runie-core/src/update/dialog/login_*` (moved)
- `crates/runie-core/src/commands/dsl/handlers/login_*` (moved)

## Notes

- Main goal is consolidation, not new features
- Follow existing DSL patterns in the codebase
