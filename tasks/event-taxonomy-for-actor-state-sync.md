# Event taxonomy for actor state sync

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P0

**Depends on**: actor-owned-state-ssot
**Blocks**: config-ssot-via-configactor, session-actor-owns-session-state, input-actor-owns-input-state, view-actor-owns-view-state, completion-actor-owns-completion-state, turn-actor-owns-agent-turn-state, permission-actor-owns-approvals, notification-actor-owns-transient-messages, trust-actor-owns-trust-decisions, env-actor-owns-git-cwd, fff-indexer-owns-file-picker-results, unified-dsl-intents-for-state-mutations, app-state-read-only-projection

## Description

The `Event` enum was a flat 109-variant bag that mixed user intents, actor facts, UI control, and IO results. This task defined a clear taxonomy: **Intents** (requests to actors) and **Facts** (broadcast state changes), plus **Control** events for lifecycle/IO. This allows the compiler and conventions to enforce that handlers do not mutate state directly.

## Acceptance criteria

- [x] Documented taxonomy: every `Event` variant is classified as Intent, Fact, or Control (lifecycle/IO).
- [x] Event classification via `Event::kind()` method returning `EventKind` enum.
- [x] Intent types are designed for the declarative DSL: `Intent` enum in `event/intent.rs` with typed variants per actor domain.
- [x] `Event::into_intent()` converts Intent events to typed `Intent`.
- [x] Control events (`Quit`, `Abort`, terminal resize, etc.) are classified as `EventKind::Control`.
- [x] Naming convention documented and enforced: intents are imperative (`SetTheme`, `SubmitInput`), facts are past-tense/descriptive (`ConfigLoaded`, `SessionChanged`).
- [x] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [x] `intent_fact_partition_is_exhaustive` — every variant is classified as Intent, Fact, or Control.
- [x] `intent_events_convert_to_intent` — Intent events return Some(Intent) from into_intent().
- [x] `fact_events_return_none_from_into_intent` — Fact events return None from into_intent().
- [x] `fact_events_are_classified` — Fact events are correctly classified.
- [x] `control_events_are_classified` — Control events are correctly classified.

### Layer 2 — Event Handling
- [x] `intent_events_have_typed_intent_conversion` — Intent events convert to typed Intent and route correctly.
- [x] `fact_events_do_not_convert_to_intent` — Fact events correctly skip intent conversion.
- [x] `all_input_events_dispatch` — input events reach the input handler.
- [x] `all_agent_events_dispatch` — agent events reach the agent handler.

### Layer 3 — Rendering
- N/A (event taxonomy is logic-layer only).

### Layer 4 — Provider Replay / Mock-Tool E2E
- N/A.

## Files touched

- `crates/runie-core/src/event/kind/mod.rs` — `EventKind` enum and `Event::kind()` implementation.
- `crates/runie-core/src/event/intent.rs` — typed `Intent` enum for declarative DSL.
- `crates/runie-core/src/event/intent_impl.rs` — `Event::into_intent()` implementation.
- `crates/runie-core/src/event/variants_tests.rs` — Layer 1/2 tests.
- `crates/runie-core/src/event/kind/mod.rs` — Layer 1 tests for taxonomy.

## Notes

The taxonomy is implemented via:
- `EventKind` enum: `Intent`, `Fact`, `Control`
- `Event::kind()` method: classifies each variant
- `Intent` enum: typed intents per actor domain
- `Event::into_intent()` method: converts Intent events to typed Intent

The flat `Event` enum is retained for backward compatibility. The `EventKind` and `Intent` types provide the type-level taxonomy without requiring a wrapper enum.

## Related

- `actor-owned-state-ssot` (done) — prerequisite for actor ownership
- `declarative-actor-dsl` (todo) — builds on this taxonomy
- `app-state-read-only-projection` (todo) — consumes Facts only
