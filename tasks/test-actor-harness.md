# Test harness for actor-based state

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Testing
**Priority**: P0

**Depends on**: actor-lifecycle-and-handle-registry, event-taxonomy-for-actor-state-sync, app-state-read-only-projection
**Blocks**: session-actor-owns-session-state, input-actor-owns-input-state, view-actor-owns-view-state, completion-actor-owns-completion-state, turn-actor-owns-agent-turn-state, permission-actor-owns-approvals, notification-actor-owns-transient-messages, trust-actor-owns-trust-decisions

## Description

Once state is owned by actors, unit tests can no longer mutate `AppState` fields directly. Provide a `TestHarness` that can spawn a lightweight actor for a single domain, feed it intents, and observe the resulting facts. The harness must make the 4-layer testing strategy from `AGENTS.md` still easy to apply.

## Acceptance criteria

- [ ] `crates/runie-core/src/testing/actor_harness.rs` (or similar) provides:
  - `TestHarness::new()` with an empty `AppState` and `TestActorSystem`.
  - `harness.send(intent)` to send an intent to the appropriate actor.
  - `harness.facts()` to read all facts emitted since the last check.
  - `harness.state()` to read the current `AppState` projection.
  - `harness.spawn_actor::<A>()` to add a real actor to the harness.
- [ ] A `TestActorSystem` records sent messages instead of dispatching them, unless a real actor is registered.
- [ ] Existing unit tests that currently set `state.config.*`, `state.session.messages`, etc. are migrated to use the harness (can be done incrementally per actor task).
- [ ] Layer-1/2 tests do not require `tokio` runtime unless they explicitly spawn an async actor.
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [ ] `harness_send_intent_records_fact` — sending `InputIntent::InsertChar` produces a recorded message.
- [ ] `harness_state_projection_updates_after_fact` — applying a `SessionChanged` fact updates the exposed state.

### Layer 2 — Event Handling
- [ ] `harness_routes_theme_intent_to_config_actor` — `ConfigIntent::SetTheme` reaches the config actor.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/testing/` — new test harness module.
- `crates/runie-core/src/actors/mod.rs` — `TestActorSystem` implementation.
- Existing test files — migrate incrementally.

## Notes

- This harness is critical for keeping tests fast and deterministic after the actor refactor.
- It should not replace the existing `runie-testing` crate; it is a core-internal helper for actor unit tests.
