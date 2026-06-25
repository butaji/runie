# Dedupe messages_changed + ensure_fresh call sequence

**Status**: done
**Milestone**: R4
**Category**: Architecture / Testing
**Priority**: P3

**Depends on**: none
**Blocks**: none

## Description

The two-call sequence `state.messages_changed(); state.ensure_fresh();` appears 174 times across `runie-core/src/tests/*` and `runie-tui/src/tests/*`. Test helpers that push messages should perform this automatically.

## Acceptance Criteria

- [x] A helper such as `AppState::refresh_after_message_change(&mut self)` replaces the two-call sequence.
- [x] Message-pushing helpers (`push_assistant`, `push_user`, etc.) call it automatically.
- [x] All 174 call sites are replaced.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `refresh_after_message_change_updates_flags` — helper sets the same flags as the two-call sequence.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/model/state/app_state.rs` or test support module
- All `crates/runie-core/src/tests/*.rs`
- All `crates/runie-tui/src/tests/*.rs`

## Notes

This is test-only duplication. Coordinate with `dedupe-fresh-state-test-helper`, which already targets shared test helpers.
