# Handle Poisoned Mutex in EventBus::publish

**Status**: done
**Milestone**: R3
**Category**: Core / State
**Priority**: P3

**Depends on**: `event-bus-replay-semantics`
**Blocks**: none

## Description

`EventBus::publish` uses `.unwrap()` on a `std::sync::Mutex`. A panicking subscriber could poison it and crash subsequent publishes.

## Acceptance Criteria

- [ ] Use `parking_lot::Mutex` (already a workspace dependency) or handle poisoning gracefully.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `publish_survives_poisoned_mutex` — publishing after a subscriber panic does not panic.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/src/bus.rs`

## Notes

Can be done together with `event-bus-replay-semantics` since both touch `bus.rs`.
