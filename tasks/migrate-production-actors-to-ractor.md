# Migrate production actors to `ractor`

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0/P1

**Depends on**: none
**Blocks**: delete-dead-actor-modules-and-custom-trait

## Description

The codebase still mixes the legacy custom `Actor` trait (`crates/runie-core/src/actors/trait.rs`) with `ractor`. Before the custom trait can be removed, every actor that is spawned in production must be migrated to `ractor`. This task converts the following actors while keeping `cargo check --workspace` green at each step:

- `ConfigActor` (`crates/runie-core/src/actors/config/actor.rs`)
- `ProviderActor` (`crates/runie-core/src/actors/provider/actor.rs`)
- `IoActor` (`crates/runie-core/src/actors/io/actor.rs`)
- `SessionActor` (`crates/runie-core/src/actors/session/actor.rs`)
- `InputActor` legacy custom implementation
- `TurnActor` legacy custom implementation (`crates/runie-core/src/actors/turn/actor.rs`)
- `AgentActor` in `runie-agent` (`crates/runie-agent/src/actor.rs`)

The custom `Actor` trait, `spawn_actor`, `GenericActorHandle`, and `Reply` are **left in place** temporarily so that unmigrated code continues to compile. Dead actors (`ViewActor`, `PlanActor`, `TrustActor`, `CompletionActor`, `UiControlActor`) are explicitly out of scope for this task.

## Acceptance Criteria

- [ ] Each production actor has a `ractor`-based implementation that compiles and passes its unit tests.
- [ ] All production spawn sites use the ractor-based actor (e.g., `Leader::start`, TUI bootstrap, CLI bootstrap).
- [ ] The legacy custom trait and helpers still exist but are no longer used by production code.
- [ ] `UiControlActor` is either migrated or explicitly removed in a follow-up task; it is not wired into the build in its current broken state.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `ractor_config_actor_state_machine` — `ConfigActor` initializes, loads, and emits `ConfigLoaded` from a ractor message handler.
- [ ] `ractor_provider_actor_builds_provider` — `ProviderActor` builds a `DynProvider` and emits `ModelsFetched` via ractor.

### Layer 2 — Event Handling
- [ ] `ractor_actor_spawn_lifecycle` — starts and stops each migrated production actor through ractor and asserts clean shutdown.
- [ ] `leader_start_uses_only_ractor_actors` — `Leader::start` instantiates ractor-based actors for the production set.

### Layer 3 — Rendering
- [ ] N/A — this task changes runtime plumbing, not widgets.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `smoke_migrated_actors_run_full_turn` — a provider-replay turn exercises the migrated `TurnActor`/`AgentActor` path end-to-end.

## Files touched

- `crates/runie-core/src/actors/config/actor.rs`
- `crates/runie-core/src/actors/provider/actor.rs`
- `crates/runie-core/src/actors/io/actor.rs`
- `crates/runie-core/src/actors/session/actor.rs`
- `crates/runie-core/src/actors/input/actor.rs`
- `crates/runie-core/src/actors/turn/actor.rs`
- `crates/runie-agent/src/actor.rs`
- `crates/runie-core/src/actors/mod.rs` (exports)
- `crates/runie-core/src/actors/handles.rs` (temporary compatibility shims)
- `crates/runie-tui/src/main.rs` and `crates/runie-cli/src/acp.rs` (spawn sites)

## Notes

- Do **not** delete `trait.rs` in this task; that is `delete-dead-actor-modules-and-custom-trait`.
- `AgentActor` lives in `runie-agent`, so this task crosses crate boundaries. Keep `runie-agent` depending on the ractor adapter in `runie-core`.
- `UiControlActor` is currently unwired and references `Event` variants that do not exist. Leave it untouched here; decide its fate after the production set is stable.
- Rejected alternative: deleting the custom trait first and fixing everything at once. That creates a long-lived broken branch and conflicts with parallel work.
