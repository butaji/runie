# Clean Up State Helpers and Dead Code

**Status**: done
**Milestone**: R3
**Category**: Core / State
**Priority**: P2

**Depends on**: (none)
**Blocks**: (none)

## Description

A number of small redundancies and dead functions add noise:

- `SpeedWindow` in `state.rs` uses `Vec::remove(0)` instead of `VecDeque`.
- `update/mod.rs` contains stub functions that always return `false` or
  `None` (`is_login_flow_event_input`, `is_providers_event_input`,
  `handle_vim_nav_event_input`).

## Acceptance Criteria

- [x] `SpeedWindow` uses `VecDeque` or a generic rolling-window helper.
- [x] Stub functions are removed or implemented.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] All SpeedWindow tests pass (15 tests including speed calculation, rolling window, token limits).

## Files touched

- `crates/runie-core/src/state.rs` — SpeedWindow now uses VecDeque
- `crates/runie-core/src/update/mod.rs` — removed unused stub functions

## Notes

- The `now()` duplicate was already resolved (only one exists now).
- Login validation simplification was marked as "where possible" - nested conditionals provide specific error messages that are valuable.
