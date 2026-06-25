# Fold State into Model State

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Refactoring
**Priority**: P2

**Depends on**: event-taxonomy-for-actor-state-sync
**Blocks**: decouple-appstate-from-view-cache, split-runie-core-into-domain-and-io-crates

## Description

Move application state from scattered locations into the `model/state/` module. Currently state is split between `AppState`, `model/state/`, and various other locations. This task consolidates state into a single coherent model.

## Acceptance Criteria

- [ ] All domain state lives in `model/state/`
- [ ] `AppState` becomes a thin wrapper/projection
- [ ] No state defined outside `model/state/`
- [ ] `cargo test --workspace` passes

## Tests

### Layer 1 — State/Logic
- [ ] `model_state_contains_all_domains`

### Layer 2 — Event Handling
- [ ] N/A

### Layer 3 — Rendering
- [ ] N/A

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A

## Files touched

- `crates/runie-core/src/model/state/`
- `crates/runie-core/src/app_state.rs` (move to model/state/)

## Notes

- This is a refactoring task
- Main goal is consolidation, not behavior change
