# Event taxonomy for actor state sync

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P0

**Depends on**: actor-owned-state-ssot
**Blocks**: config-ssot-via-configactor, session-actor-owns-session-state, input-actor-owns-input-state, view-actor-owns-view-state, completion-actor-owns-completion-state, turn-actor-owns-agent-turn-state, permission-actor-owns-approvals, notification-actor-owns-transient-messages, trust-actor-owns-trust-decisions, env-actor-owns-git-cwd, fff-indexer-owns-file-picker-results, unified-dsl-intents-for-state-mutations, app-state-read-only-projection

## Description

The `Event` enum is a flat 109-variant bag. It mixes user intents, actor facts, UI control, and IO results. Define a clear taxonomy: **Intents** (requests to actors) and **Facts** (broadcast state changes). This lets the compiler and conventions enforce that handlers do not mutate state directly.

## Acceptance criteria

- [ ] Documented taxonomy: every `Event` variant is classified as Intent, Fact, or Control (lifecycle/IO).
- [ ] New top-level wrapper or sub-enums: `Event::Intent(Intent)` and `Event::Fact(Fact)` (or actor-specific intent enums).
- [ ] Intent types are designed for the declarative DSL: each actor exposes a typed intent enum that implements `Into<Intent>`.
- [ ] Existing actor messages (`ConfigMsg`, `SessionMsg`, etc.) are derived from intent variants or share a 1:1 mapping.
- [ ] Handlers/commands produce only `Intent` events; `AppState::update` consumes only `Fact` events.
- [ ] Control events (`Quit`, `Abort`, terminal resize, etc.) are routed without mutating owned state, or are converted to intents where appropriate.
- [ ] Naming convention documented and enforced: intents are imperative (`SetTheme`, `SubmitInput`), facts are past-tense/descriptive (`ConfigLoaded`, `SessionChanged`).
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [ ] `intent_fact_partition_is_exhaustive` — every variant is classified.

### Layer 2 — Event Handling
- [ ] `intent_event_does_not_update_app_state_directly` — `AppState::update` ignores or panics on intent events in debug builds.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/event/variants.rs` — refactor into intent/fact/control groups.
- `crates/runie-core/src/event/mod.rs` — expose new enums.
- `crates/runie-core/src/update/mod.rs` — dispatch intents to actors, facts to projection.
- `crates/runie-core/src/model/state/app_state.rs` — projection consumes facts only.
- All command/dialog/input handlers — update event construction.

## Notes

- This is the foundational task for the whole actor-ownership program. Do it first.
- A mechanical first step: rename the existing `Event` enum to `RawEvent`, wrap it in `Event { Intent(Intent) | Fact(Fact) | Control(Control) }`, then migrate.
- Keep the migration incremental to avoid a giant PR.
