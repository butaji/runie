# Migrate production actors to `ractor`

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0/P1

**Depends on**: none
**Blocks**: delete-dead-actor-modules-and-custom-trait

## Description

The codebase still mixes the legacy custom `Actor` trait (`crates/runie-core/src/actors/trait.rs`) with `ractor`. Before the custom trait can be removed, every actor that is spawned in production must run on `ractor`.

Current state as of this review:

- **Already migrated:** `InputActor` (`crates/runie-core/src/actors/input/actor.rs`) and `RactorPermissionActor` (`crates/runie-core/src/actors/permission/ractor_permission.rs`).
- **Ractor implementation exists but is not wired to production:** `RactorConfigActor` (`crates/runie-core/src/actors/config/ractor_config.rs`) is exported but every spawn site still uses the legacy `ConfigActor::spawn`.
- **Still custom-trait in production:** `ProviderActor`, `IoActor`, `SessionActor`, `FffIndexerActor`, and `AgentActor` (`crates/runie-agent/src/actor.rs`).

This task converts the remaining custom-trait production actors while keeping `cargo check --workspace` green at each step. The legacy `Actor` trait, `spawn_actor`, `GenericActorHandle`, and `Reply` are **left in place** temporarily so that unmigrated code continues to compile. Dead actors (`ViewActor`, `PlanActor`, `TrustActor`, `CompletionActor`, `UiControlActor`) are explicitly out of scope.

## Acceptance Criteria

- [ ] `RactorConfigActor` is wired into production spawn sites and the legacy `ConfigActor` is no longer spawned outside tests.
- [ ] `ProviderActor`, `IoActor`, `SessionActor`, `FffIndexerActor`, and `AgentActor` each have a `ractor`-based implementation used in production.
- [ ] `HeadlessRuntime` (`crates/runie-core/src/headless_runtime.rs`) is updated to use the ractor-based actors instead of the legacy ones.
- [ ] All production spawn sites (`runie-tui/src/main.rs`, `runie-cli/src/acp.rs`, `Leader::start`, `HeadlessRuntime`) use the ractor-based versions.
- [ ] The legacy custom trait and helpers still exist but are no longer used by production code.
- [ ] `UiControlActor` is left untouched; its fate is decided in `delete-dead-actor-modules-and-custom-trait`.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `ractor_config_actor_state_machine` — `RactorConfigActor` initializes, loads, and emits `ConfigLoaded` from a ractor message handler.
- [ ] `ractor_provider_actor_builds_provider` — `ProviderActor` builds a `DynProvider` and emits `ModelsFetched` via ractor.
- [ ] `ractor_fff_indexer_actor_searches` — `FffIndexerActor` accepts a search request through ractor and returns results.

### Layer 2 — Event Handling
- [ ] `ractor_actor_spawn_lifecycle` — starts and stops each migrated production actor through ractor and asserts clean shutdown.
- [ ] `leader_start_uses_only_ractor_actors` — `Leader::start` instantiates ractor-based actors for the production set.
- [ ] `headless_runtime_uses_ractor_actors` — `HeadlessRuntime` starts without legacy `Actor` trait actors.

### Layer 3 — Rendering
- [ ] N/A — this task changes runtime plumbing, not widgets.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `smoke_migrated_actors_run_full_turn` — a provider-replay turn exercises the migrated `TurnActor`/`AgentActor` path end-to-end.

## Files touched

- `crates/runie-core/src/actors/config/actor.rs`
- `crates/runie-core/src/actors/config/ractor_config.rs`
- `crates/runie-core/src/actors/provider/actor.rs`
- `crates/runie-core/src/actors/io/actor.rs`
- `crates/runie-core/src/actors/session/actor.rs`
- `crates/runie-core/src/actors/input/actor.rs`
- `crates/runie-core/src/actors/turn/actor.rs`
- `crates/runie-core/src/actors/fff_indexer/mod.rs`
- `crates/runie-agent/src/actor.rs`
- `crates/runie-core/src/actors/mod.rs` (exports)
- `crates/runie-core/src/actors/handles.rs` (temporary compatibility shims)
- `crates/runie-core/src/headless_runtime.rs`
- `crates/runie-tui/src/main.rs`
- `crates/runie-cli/src/acp.rs`
- `crates/runie-core/src/actors/leader/actor.rs` (spawn list)

## Notes

- Do **not** delete `trait.rs` in this task; that is `delete-dead-actor-modules-and-custom-trait`.
- `AgentActor` lives in `runie-agent`, so this task crosses crate boundaries. Keep `runie-agent` depending on the ractor adapter in `runie-core`.
- `UiControlActor` is currently unwired and references `Event` variants that do not exist. Leave it untouched here.
- The switch from legacy `ConfigActor` to `RactorConfigActor` is the highest-impact single change because it is spawned in the TUI, CLI, leader, and headless runtime.
- Rejected alternative: deleting the custom trait first and fixing everything at once. That creates a long-lived broken branch and conflicts with parallel work.
