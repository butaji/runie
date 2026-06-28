# Migrate production actors to `ractor`

**Status**: partial
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0/P1

**Depends on**: none
**Blocks**: delete-dead-actor-modules-and-custom-trait

## Description

The codebase still mixes the legacy custom `Actor` trait (`crates/runie-core/src/actors/trait.rs`) with `ractor`. Before the custom trait can be removed, every actor that is spawned in production must run on `ractor`.

Current state as of Round 2 (2026-06-28):

- **Already migrated and wired to production:** `InputActor`, `RactorPermissionActor`, `RactorConfigActor`, `RactorIoActor`, and `RactorSessionActor`.
  - `RactorConfigActor` is spawned in `runie-tui/src/main.rs`, `runie-cli/src/acp.rs`, `crates/runie-core/src/headless_runtime.rs`, and `crates/runie-core/src/actors/leader/actor.rs`.
  - `RactorIoActor` is spawned in `runie-tui/src/main.rs` and `crates/runie-core/src/actors/leader/actor.rs`.
  - `RactorSessionActor` is spawned in `runie-tui/src/main.rs` and `crates/runie-core/src/actors/leader/actor.rs`.
- **Still custom-trait in production:** `ProviderActor`, `FffIndexerActor`, and `AgentActor` (`crates/runie-agent/src/actor.rs`).

This task finishes the migration in phases so that `cargo check --workspace` stays green after each step. The legacy `Actor` trait, `spawn_actor`, `GenericActorHandle`, and `Reply` are **left in place** temporarily so that unmigrated code continues to compile. Dead actors (`ViewActor`, `PlanActor`, `TrustActor`, `CompletionActor`, `UiControlActor`) are explicitly out of scope.

### Migration progress (Round 2)

| Actor | Ractor impl | Wired to production |
|-------|-------------|----------------------|
| ConfigActor | ✓ `RactorConfigActor` | ✓ |
| PermissionActor | ✓ `RactorPermissionActor` | ✓ |
| InputActor | ✓ `RactorInputHandle` | ✓ |
| TurnActor | ✓ `RactorTurnActor` | ✓ |
| IoActor | ✓ `RactorIoActor` | ✓ |
| SessionActor | ✓ `RactorSessionActor` | ✓ |
| ProviderActor | ✗ | ✗ |
| FffIndexerActor | ✗ | ✗ |
| AgentActor | ✗ | ✗ |

### Suggested phases (Pareto ordering)

1. **Migrate `ProviderActor`** — highest-impact remaining actor; referenced by leader, headless runtime, TUI, and CLI.
2. **Migrate `FffIndexerActor`**; keep a small wrapper if the static `FffSearchState` service-locator remains.
3. **Migrate `AgentActor`** in `runie-agent`; introduce a factory trait in `runie-core` to avoid a crate-dependency cycle.

## Acceptance Criteria

- [x] `RactorConfigActor` is wired into production spawn sites and the legacy `ConfigActor` is no longer spawned outside tests.
- [x] `RactorIoActor` is wired into production spawn sites (TUI, Leader).
- [x] `RactorSessionActor` is wired into production spawn sites (TUI, Leader).
- [ ] `ProviderActor` has a `ractor`-based implementation used in production.
- [ ] `FffIndexerActor` has a `ractor`-based implementation used in production.
- [ ] `AgentActor` has a `ractor`-based implementation used in production.
- [ ] All production spawn sites (`runie-tui/src/main.rs`, `runie-cli/src/acp.rs`, `Leader::start`, `HeadlessRuntime`) use the ractor-based versions for all migrated actors.
- [ ] `HeadlessRuntime` (`crates/runie-core/src/headless_runtime.rs`) is updated to use the ractor-based actors instead of the legacy ones.
- [ ] The legacy custom trait and helpers still exist but are no longer used by production code.
- [ ] `UiControlActor` is left untouched; its fate is decided in `delete-dead-actor-modules-and-custom-trait`.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `ractor_io_actor_spawns` — `IoActor` spawns through ractor.
- [x] `ractor_io_receives_messages` — `IoActor` handles messages through ractor.
- [x] `ractor_session_actor_spawns` — `SessionActor` spawns through ractor.
- [x] `ractor_session_handles_trust_loaded` — `SessionActor` emits TrustLoaded on spawn.
- [x] `ractor_session_adds_user_message` — `SessionActor` handles AddUserMessage through ractor.
- [ ] `ractor_provider_actor_builds_provider` — `ProviderActor` builds a `DynProvider` and emits `ModelsFetched` via ractor.
- [ ] `ractor_fff_indexer_actor_searches` — `FffIndexerActor` accepts a search request through ractor and returns results.

### Layer 2 — Event Handling
- [x] `leader_start_uses_ractor_io` — `Leader::start` instantiates `RactorIoActor`.
- [x] `leader_start_uses_ractor_session` — `Leader::start` instantiates `RactorSessionActor`.
- [ ] `ractor_actor_spawn_lifecycle` — starts and stops each migrated production actor through ractor and asserts clean shutdown.
- [ ] `headless_runtime_uses_ractor_actors` — `HeadlessRuntime` starts without legacy `Actor` trait actors.

### Layer 3 — Rendering
- [ ] N/A — this task changes runtime plumbing, not widgets.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `smoke_migrated_actors_run_full_turn` — a provider-replay turn exercises the migrated `TurnActor`/`AgentActor` path end-to-end.

## Files touched

### Round 1-2 (completed)
- `crates/runie-core/src/actors/io/ractor_io.rs` (new)
- `crates/runie-core/src/actors/io/mod.rs` (exports)
- `crates/runie-core/src/actors/session/ractor_session.rs` (new, with submodules)
- `crates/runie-core/src/actors/session/ractor_session_actor.rs` (new)
- `crates/runie-core/src/actors/session/ractor_session_handle.rs` (new)
- `crates/runie-core/src/actors/session/mod.rs` (exports)
- `crates/runie-core/src/actors/mod.rs` (exports)
- `crates/runie-core/src/actors/handles.rs` (updated)
- `crates/runie-core/src/actors/leader/actor.rs` (spawn list and handle types)
- `crates/runie-tui/src/main.rs` (spawn calls)
- `crates/runie-tui/src/ui_actor.rs` (handle types)
- `crates/runie-core/src/commands/dsl/handlers/session/run.rs` (handle type)

### Remaining
- `crates/runie-core/src/actors/provider/ractor_provider.rs` (new)
- `crates/runie-core/src/actors/provider/actor.rs` (mark deprecated)
- `crates/runie-core/src/actors/fff_indexer/ractor_fff_indexer.rs` (new)
- `crates/runie-core/src/actors/fff_indexer/mod.rs` (exports)
- `crates/runie-agent/src/actor.rs` (ractor migration)
- `crates/runie-core/src/headless_runtime.rs` (update spawn calls)
- `crates/runie-cli/src/acp.rs` (update spawn calls)

## Notes

- Do **not** delete `trait.rs` in this task; that is `delete-dead-actor-modules-and-custom-trait`.
- `AgentActor` lives in `runie-agent`, so this task crosses crate boundaries. Keep `runie-agent` depending on the ractor adapter in `runie-core`.
- `UiControlActor` is currently unwired and its module is not included in `actors/mod.rs`, so it does not affect compilation. Leave its deletion to `delete-dead-actor-modules-and-custom-trait`.
- After the migration, `Reply` must be moved out of `trait.rs` (e.g., to `actors/ractor_adapter.rs` or `actors/reply.rs`) before `trait.rs` can be deleted.
- Rejected alternative: deleting the custom trait first and fixing everything at once. That creates a long-lived broken branch and conflicts with parallel work.
