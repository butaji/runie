# Dedupe ensure_turn_complete_last calls

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/update/agent/mod.rs` calls `state.ensure_turn_complete_last();` in 6 of 9 match arms when handling `AgentEvent`. It is easy to forget the call when adding a new event variant, breaking the invariant that `TurnComplete` must be last.

## Acceptance Criteria

- [ ] A post-match helper or wrapper ensures the invariant automatically for variants that need it.
- [ ] The explicit calls in individual arms are removed.
- [ ] The invariant is still enforced.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `turn_complete_remains_last_after_any_event` — property test feeding random event sequences.

### Layer 2 — Event Handling
- [ ] `turn_complete_event_kept_last` — dispatch `TurnComplete` plus other events and assert order.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/update/agent/mod.rs`

## Notes

If only some variants need the call, define an `EventNeedsReorder` predicate rather than calling after every arm.
