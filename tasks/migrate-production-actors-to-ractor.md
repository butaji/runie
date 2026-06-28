# Migrate production actors to `ractor`

**Status**: partial
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0/P1

**Depends on**: none
**Blocks**: delete-dead-actor-modules-and-custom-trait

## Description

The codebase still mixes the legacy custom `Actor` trait (`crates/runie-core/src/actors/trait.rs`) with `ractor`. Before the custom trait can be removed, every actor that is spawned in production must run on `ractor`.

Current state as of this review:

- **Already migrated and wired:** `InputActor` (`crates/runie-core/src/actors/input/actor.rs`) and `RactorPermissionActor` (`crates/runie-core/src/actors/permission/ractor_permission.rs`).
- **Implemented but not wired to production:** `RactorConfigActor` (`crates/runie-core/src/actors/config/ractor_config.rs`) is implemented and exported (`crates/runie-core/src/actors/config/mod.rs`, `crates/runie-core/src/actors/mod.rs`), but every production spawn site still uses the legacy `ConfigActor::spawn`.
- **Still custom-trait in production:** `ProviderActor`, `IoActor`, `SessionActor`, `FffIndexerActor`, and `AgentActor` (`crates/runie-agent/src/actor.rs`).

This task finishes the migration in phases so that `cargo check --workspace` stays green after each step. The legacy `Actor` trait, `spawn_actor`, `GenericActorHandle`, and `Reply` are **left in place** temporarily so that unmigrated code continues to compile. Dead actors (`ViewActor`, `PlanActor`, `TrustActor`, `CompletionActor`, `UiControlActor`) are explicitly out of scope.

### Suggested phases (Pareto ordering)

1. **Wire `RactorConfigActor` to production** — switch TUI, CLI, `Leader::start`, and `HeadlessRuntime` from legacy `ConfigActor` to `RactorConfigActor`. This is the highest-impact single change.
2. **Migrate `ProviderActor`** — the next most referenced production actor.
3. **Migrate `IoActor` and `SessionActor`** in either order.
4. **Migrate `FffIndexerActor`**; keep a small wrapper if the static `FffSearchState` service-locator remains.
5. **Migrate `AgentActor`** in `runie-agent`; introduce a factory trait in `runie-core` to avoid a crate-dependency cycle.

## Acceptance Criteria

- [ ] `RactorConfigActor` is wired into production spawn sites and the legacy `ConfigActor` is no longer spawned outside tests.
- [ ] `ProviderActor`, `IoActor`, `SessionActor`, `FffIndexerActor`, and `AgentActor` each have a `ractor`-based implementation used in production.
- [ ] All production spawn sites (`runie-tui/src/main.rs`, `runie-cli/src/acp.rs`, `Leader::start`, `HeadlessRuntime`) use the ractor-based versions.
- [ ] `HeadlessRuntime` (`crates/runie-core/src/headless_runtime.rs`) is updated to use the ractor-based actors instead of the legacy ones.
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
- `UiControlActor` is currently unwired and its module is not included in `actors/mod.rs`, so it does not affect compilation. Leave its deletion to `delete-dead-actor-modules-and-custom-trait`.
- The switch from legacy `ConfigActor` to `RactorConfigActor` is the highest-impact single change because it is spawned in the TUI, CLI, leader, and headless runtime.
- `RactorConfigActor` already exists; this task is partially complete. The remaining work is wiring it to production and migrating the other actors.
- After the migration, `Reply` must be moved out of `trait.rs` (e.g., to `actors/ractor_adapter.rs` or `actors/reply.rs`) before `trait.rs` can be deleted.
- Rejected alternative: deleting the custom trait first and fixing everything at once. That creates a long-lived broken branch and conflicts with parallel work.
