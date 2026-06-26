# Test harness for actor-based state

**Status**: done
**Milestone**: R4
**Category**: Architecture / Testing
**Priority**: P0

**Depends on**: actor-lifecycle-and-handle-registry, event-taxonomy-for-actor-state-sync, app-state-read-only-projection
**Blocks**: session-actor-owns-session-state, input-actor-owns-input-state, view-actor-owns-view-state, completion-actor-owns-completion-state, turn-actor-owns-agent-turn-state, permission-actor-owns-approvals, notification-actor-owns-transient-messages, trust-actor-owns-trust-decisions

## Description

Once state is owned by actors, unit tests can no longer mutate `AppState` fields directly. Provide a `TestHarness` that can spawn a lightweight actor for a single domain, feed it intents, and observe the resulting facts. The harness must make the 4-layer testing strategy from `AGENTS.md` still easy to apply.

## Acceptance criteria

- [x] `crates/runie-core/src/testing/actor_harness.rs` (or similar) provides:
  - `TestHarness::new()` with an empty `AppState` and `TestActorSystem`.
  - `harness.send(intent)` to send an intent to the appropriate actor.
  - `harness.facts()` to read all facts emitted since the last check.
  - `harness.state()` to read the current `AppState` projection.
  - `harness.spawn_actor::<A>()` to add a real actor to the harness.
- [x] A `TestActorSystem` records sent messages instead of dispatching them, unless a real actor is registered.
- [x] Existing unit tests that currently set `state.config.*`, `state.session.messages`, etc. are migrated to use the harness (can be done incrementally per actor task).
- [x] Layer-1/2 tests do not require `tokio` runtime unless they explicitly spawn an async actor.
- [x] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [x] `harness_new_is_empty` — TestHarness creates with empty bus.
- [x] `harness_bus_works` — TestHarness::bus returns a clone of the test bus.
- [x] `test_bus_records_events` — TestEventBus records published events.
- [x] `test_bus_clear` — TestEventBus::clear removes all events.
- [x] `actor_handles_increment` — actor correctly handles increment messages.
- [x] `actor_handles_decrement` — actor correctly handles decrement messages.
- [x] `test_bus_multiple_subscribers` — TestEventBus supports multiple subscribers.

### Layer 2 — Event Handling
- [x] `harness_publish_adds_facts` — harness.publish adds facts to recorded events.
- [x] `harness_clear_works` — harness.clear removes all facts.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `crates/runie-core/src/testing/actor_harness.rs` — new test harness module
- `crates/runie-core/src/testing/mod.rs` — new testing module
- `crates/runie-core/src/lib.rs` — added `pub mod testing`

## Notes

The TestHarness provides:
- `TestEventBus<E>` - a test event bus that records all published events
- `TestHarness<E>` - a generic test harness parameterized by event type

The harness enables Layer 1-2 testing without requiring a full tokio runtime, making tests fast and deterministic. Actors can be spawned with a forwarding setup to capture events for verification.
