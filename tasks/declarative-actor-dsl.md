# Declarative Actor DSL

**Status**: todo
**Milestone": R5
**Category": Architecture / Actors
**Priority": P2

**Depends on": actor-owned-state-ssot, event-taxonomy-for-actor-state-sync
**Blocks": unified-dsl-intents-for-state-mutations

## Description

Define a declarative DSL for defining actors and their message handlers. The goal is to reduce boilerplate in actor implementations.

## Acceptance Criteria

- [ ] `define_actor!` macro defined
- [ ] Message handlers use declarative syntax
- [ ] `cargo test --workspace` passes

## Tests

### Layer 1 — State/Logic
- [ ] `actor_macro_reduces_boilerplate`

### Layer 2 — Event Handling
- [ ] N/A

### Layer 3 — Rendering
- [ ] N/A

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A

## Files touched

- `crates/runie-macros/src/`
- `crates/runie-core/src/actors/`

## Notes

- This is a future enhancement task
- Not blocking current functionality
