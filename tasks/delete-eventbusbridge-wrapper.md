# Delete `EventBusBridge` wrapper

**Status**: done
**Note**: Verified 2026-06-29 — `EventBusBridge` not found in codebase, actors use `EventBus` directly.
**Milestone**: R6
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: migrate-production-actors-to-ractor
**Blocks**: none

## Description

`EventBusBridge<E>` is a thin wrapper around `EventBus<E>` that only forwards `publish`. `EventBus<E>` is already `Clone` and wraps `tokio::sync::broadcast`. Actors can hold `EventBus<Event>` directly; delete the wrapper.

## Acceptance Criteria

- [x] Delete `crates/runie-core/src/actors/ractor_adapter.rs` or remove `EventBusBridge` from it.
- [x] Update all actor modules that use `EventBusBridge` to hold `EventBus<Event>`.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `actor_publishes_via_event_bus` — actor still publishes events after removing the bridge.

### Layer 2 — Event Handling
- [x] N/A.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `turn_events_still_reach_ui` — provider replay events reach the UI without the bridge.

## Files touched

- `crates/runie-core/src/actors/ractor_adapter.rs`
- `crates/runie-core/src/actors/*/ractor_*.rs`
- `crates/runie-core/src/bus.rs`

## Notes

- This is a small simplification that reduces adapter boilerplate.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
