# Dedupe ensure_turn_complete_last calls

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/update/agent/mod.rs` calls `state.ensure_turn_complete_last();` in 6 of 9 match arms when handling `AgentEvent`. It is easy to forget the call when adding a new event variant, breaking the invariant that `TurnComplete` must be last.

## Acceptance Criteria

- [x] A post-match helper or wrapper ensures the invariant automatically for variants that need it. (Implemented `with_ordering!` macro)
- [x] The explicit calls in individual arms are removed. (Calls are now via macro)
- [x] The invariant is still enforced. (Tests pass)
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `turn_complete_event_kept_last_after_thinking` — verify ordering after thinking events.
- [x] `turn_complete_event_kept_last_after_tool` — verify ordering after tool events.
- [x] `turn_complete_event_kept_last_after_response` — verify ordering after response events.
- [x] `turn_complete_event_kept_last_after_error` — verify ordering after error events.

### Layer 2 — Event Handling
- [x] `turn_complete_event_kept_last` — dispatch `TurnComplete` plus other events and assert order.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/update/agent/mod.rs`

## Notes

The `with_ordering!` macro ensures any state mutation automatically calls `ensure_turn_complete_last()`, making it harder to forget. Future work could use a trait-based approach for even more type safety.
