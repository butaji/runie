# Centralize AppState ownership in UiActor

**Status**: done  
**Milestone**: R4  
**Category**: Architecture / Actors  
**Priority**: P0  

**Depends on**: arch-guardrails-enforce-3-layers  
**Blocks**: remove-io-from-runie-core  

## Description

`AppState` was mutated from multiple places: `AgentActorHandle::run_if_queued`, `app_init::bootstrap`, and `init_terminal_state`. This task made `UiActor` the sole runtime owner of `AppState` by converting all other call sites into event producers.

## Acceptance Criteria

- [x] `AgentActorHandle::run_if_queued(&mut AppState)` is removed.
- [x] `AgentActor` exposes only `run(command)`; `UiActor` decides when to start a turn via `start_next_turn_if_queued`.
- [x] `app_init::bootstrap` no longer mutates `AppState`; it emits `GitDetected`, `SkillsLoaded`, and `AuthProvidersLoaded` events.
- [x] `init_terminal_state` no longer mutates `AppState`; `TerminalSize` is emitted as the first event.
- [x] `AppState::update` routes bootstrap events through `update/bootstrap.rs`.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `apply_git_detected_sets_git_info_and_cwd_name` — `AppState` updates `git_info`/`cwd_name` via event.
- [x] `apply_skills_loaded_sets_skills` — `AppState` updates `skills` via event.
- [x] `apply_auth_providers_loaded_sets_providers` — `AppState` updates auth providers via event.

### Layer 2 — Event Handling
- [x] `start_next_turn_if_queued_pops_and_runs` — `UiActor` signals readiness and starts a turn without `AgentActorHandle` mutating state.
- [x] `start_next_turn_if_queued_noop_when_queue_empty` — empty queue is a no-op.
- [x] `start_next_turn_if_queued_noop_when_turn_active` — active turn blocks re-entry.
- [x] `terminal_size_event_sets_size` — `TerminalSize` event updates terminal dimensions.

### Layer 3 — Rendering
- [x] `settings_dialog_renders_after_config_loaded` — existing coverage.

### Layer 4 — Smoke / Crash
- [x] `smoke_mock_turn_runs_through_actor_runtime` — existing coverage.

## Files touched

- `crates/runie-agent/src/actor.rs`
- `crates/runie-tui/src/app_init.rs`
- `crates/runie-tui/src/main.rs`
- `crates/runie-tui/src/ui_actor.rs`
- `crates/runie-core/src/event/variants.rs`
- `crates/runie-core/src/update/mod.rs`
- `crates/runie-core/src/update/dispatch.rs`
- `crates/runie-core/src/update/bootstrap.rs` (new)
- `crates/runie-core/src/skills/mod.rs`
- `crates/runie-core/src/snapshot.rs`
- `crates/runie-core/src/event/variants_tests.rs`
- `crates/runie-core/src/update/input/tests.rs`

## Notes

- The `run_if_queued` logic was moved into `UiActor::start_next_turn_if_queued` because `UiActor` is the designated state owner.
- Some command handlers still take `&mut AppState`; they remain in the `legacy_app_state_mutation_files()` allow-list and will be addressed when commands are converted to event producers.
- `std::env::current_dir()` is still used inside `UiActor::handle_event` for trust persistence; that moves to a `WorkingDirSet` event in Phase 2.
