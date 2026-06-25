# Unify Persistence Actors

**Status**: todo
**Milestone": R4
**Category": Architecture / Actors
**Priority": P2

**Depends on": event-taxonomy-for-actor-state-sync
**Blocks": simplify-actor-trait

## Description

Unify persistence actors (`SessionActor`, `ConfigActor`) under a common pattern. Currently they have slightly different interfaces.

## Acceptance Criteria

- [ ] Common persistence trait defined
- [ ] `SessionActor` and `ConfigActor` implement it
- [ ] `cargo test --workspace` passes

## Tests

### Layer 1 — State/Logic
- [ ] `persistence_trait_implemented`

### Layer 2 — Event Handling
- [ ] N/A

### Layer 3 — Rendering
- [ ] N/A

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A

## Files touched

- `crates/runie-core/src/actors/config/`
- `crates/runie-core/src/actors/session/`

## Notes

- Simplification task
- May not be needed if actors are already unified enough
