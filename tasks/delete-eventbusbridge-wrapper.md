# Delete `EventBusBridge` wrapper

**Status**: done
**Milestone**: R6
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: migrate-production-actors-to-ractor
**Blocks**: none

## Description

`EventBusBridge<E>` is a thin wrapper around `EventBus<E>` that only forwards `publish`. `EventBus<E>` is already `Clone` and wraps `tokio::sync::broadcast`. Actors can hold `EventBus<Event>` directly; delete the wrapper.

## Acceptance Criteria

- [ ] Delete `crates/runie-core/src/actors/ractor_adapter.rs` or remove `EventBusBridge` from it.
- [ ] Update all actor modules that use `EventBusBridge` to hold `EventBus<Event>`.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `actor_publishes_via_event_bus` — actor still publishes events after removing the bridge.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `turn_events_still_reach_ui` — provider replay events reach the UI without the bridge.

## Files touched

- `crates/runie-core/src/actors/ractor_adapter.rs`
- `crates/runie-core/src/actors/*/ractor_*.rs`
- `crates/runie-core/src/bus.rs`

## Notes

- This is a small simplification that reduces adapter boilerplate.
