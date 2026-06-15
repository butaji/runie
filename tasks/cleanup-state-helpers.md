# Clean Up State Helpers and Dead Code

**Status**: todo
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P2

**Depends on**: (none)
**Blocks**: (none)

## Description

A number of small redundancies and dead functions add noise:

- `now()` is defined in both `runie-core/src/message.rs` and
  `runie-core/src/update/mod.rs`.
- `SpeedWindow` in `state.rs` uses `Vec::remove(0)` instead of `VecDeque`.
- `update/mod.rs` contains stub functions that always return `false` or
  `None` (`is_login_flow_event_input`, `is_providers_event_input`,
  `handle_vim_nav_event_input`).
- Login-flow validation (`login_flow/validation.rs`) is 473 lines of nested
  conditionals that could be table-driven.

## Acceptance Criteria

- [ ] A single `time::now()` utility exists in `runie-core`.
- [ ] `SpeedWindow` uses `VecDeque` or a generic rolling-window helper.
- [ ] Stub functions are removed or implemented.
- [ ] Login validation is simplified where possible without behavior change.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `speed_window_evicts_old_events` — rolling window evicts events
  outside the configured token budget.
- [ ] `now_is_single_function` — only one `now()` definition exists in
  `runie-core`.

## Files touched

- `crates/runie-core/src/message.rs`
- `crates/runie-core/src/update/mod.rs`
- `crates/runie-core/src/state.rs`
- `crates/runie-core/src/login_flow/validation.rs`

## Notes

This is a cleanup pass. Each change should be small and behavior-preserving.
