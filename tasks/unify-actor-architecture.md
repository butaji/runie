# Unify architecture around event-based actors

**Status**: superseded
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: none
**Blocks**: none 

## Description

Move the codebase to a single actor model: state lives in actors, IO is async or event-based, and domain/UI logic is not spread across `update/`, `commands/`, `login_config.rs`, and TUI entry points.

## Acceptance Criteria

- [ ] One `ConfigActor` owns `~/.runie/config.toml` reads/writes/reloads.
- [ ] One `ProviderActor` owns provider construction, validation, and model listing.
- [ ] `AgentActor` owns the turn loop and asks `ProviderActor` for a provider.
- [ ] `UiActor` owns only the UI projection (`AppState`/`Snapshot`).
- [ ] Domain logic stays pure; rendering stays `Snapshot -> Frame`.
- [ ] All workspace tests pass.

## Tests

### Layer 1 — State/Logic
- [ ] `config_actor_loads_and_emits_config_loaded`
- [ ] `provider_actor_builds_provider_from_config`
- [ ] `apply_config_sets_active_model_from_config`

### Layer 2 — Event Handling
- [ ] `config_loaded_event_updates_keybindings_and_prompts`
- [ ] `save_provider_event_persists_to_config_file`

### Layer 3 — Rendering
- [ ] `settings_dialog_renders_after_config_loaded`
- [ ] `providers_dialog_renders_from_config_cache`

### Layer 4 — Smoke / Crash
- [ ] `smoke_mock_turn_runs_through_actor_runtime`

## Files touched

- `crates/runie-core/src/actors/config/` (new)
- `crates/runie-core/src/actors/provider.rs` (new)
- `crates/runie-core/src/actors/mod.rs`
- `crates/runie-core/src/event/variants.rs`
- `crates/runie-core/src/model/state/app_state.rs`
- `crates/runie-core/src/update/mod.rs`
- `crates/runie-core/src/update/system.rs`
- `crates/runie-core/src/update/system/model.rs`
- `crates/runie-core/src/update/login_flow.rs`
- `crates/runie-core/src/commands/dsl/handlers/system.rs`
- `crates/runie-tui/src/main.rs`
- `crates/runie-tui/src/app_init.rs`
- `crates/runie-tui/src/ui_actor.rs`
- `docs/Architecture.md`

## Notes

Phased approach:
1. Extract `ConfigActor` and route config-driven init through `Event::ConfigLoaded`.
2. Extract `ProviderActor` and route all `DynProvider::new_with_config` calls to it.
3. Convert the plain `agent_loop` task into a real `AgentActor`.
4. Clean remaining MVU violations and unify startup/init.
5. Update `docs/Architecture.md`; archive other docs leaving only `Architecture.md` and `UI_UX.md`.
