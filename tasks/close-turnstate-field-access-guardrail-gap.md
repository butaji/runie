# Close `turn_state` field-access guardrail gap

**Status**: todo
**Milestone**: R7
**Category**: Core / State
**Priority**: P1

**Depends on**: none
**Blocks**: remove-turnstate-from-appstate

## Description

`crates/runie-core/build.rs` `APPSTATE_PATTERNS` does not include `state.turn_state.` or `self.turn_state.`, so direct field access to the authoritative `TurnState` inside `AppState` is not caught. Examples in production code:

- `crates/runie-core/src/update/agent/core/mod.rs:11`
- `crates/runie-core/src/update/agent/core_messages.rs:36`
- `crates/runie-core/src/model/state/turn_projections.rs:283`

## Acceptance Criteria

- [x] Add `state.turn_state.` and `self.turn_state.` patterns to `APPSTATE_PATTERNS` in `build.rs`.
- [x] Refactor all production call sites to use accessors or event-driven projections.
- [x] `cargo test --workspace` passes.
- [x] `cargo check --workspace` passes with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `build_script_catches_turn_state_direct_access` — new lint pattern flags direct `turn_state` field access.

### Layer 2 — Event Handling
- [x] N/A — guardrail concern.

### Layer 3 — Rendering
- [x] N/A — guardrail concern.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A — guardrail concern.

### Live Tmux Testing Session
- [x] Run a full TUI turn after refactoring call sites to confirm behavior is unchanged.

## Files touched

- `crates/runie-core/build.rs`
- `crates/runie-core/src/update/agent/core/mod.rs`
- `crates/runie-core/src/update/agent/core_messages.rs`
- `crates/runie-core/src/model/state/turn_projections.rs`

## Notes

- This task becomes moot if `remove-turnstate-from-appstate.md` is completed first, because `turn_state` will no longer exist on `AppState`. Coordinate ordering; either do this first as a quick win, or fold it into the larger removal task.
