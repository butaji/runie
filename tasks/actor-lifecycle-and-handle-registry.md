# Actor lifecycle and handle registry

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: actor-owned-state-ssot
**Blocks**: config-ssot-via-configactor, session-actor-owns-session-state, input-actor-owns-input-state, view-actor-owns-view-state, completion-actor-owns-completion-state, turn-actor-owns-agent-turn-state, permission-actor-owns-approvals, notification-actor-owns-transient-messages, trust-actor-owns-trust-decisions, env-actor-owns-git-cwd, fff-indexer-owns-file-picker-results, ui-control-actor-owns-dialog-state

## Description

The new actors need a clear lifecycle: where they are spawned, how their handles reach `AppState`, and how the TUI/bootstrap wires them together.

Current reality (`runie-tui/src/main.rs`):
- `bootstrap_app()` spawns `ConfigActor`, `ProviderActor`, `PersistenceActor`, `SessionStoreActor`, `IoActor` and injects their senders/handles into `AppState`.
- `spawn_background_tasks()` spawns `AgentActor` and `UiActor`.
- `spawn_session_persistence()` spawns the existing broadcast-subscriber `SessionActor` (durability logger).
- `FffIndexerActor` is **not spawned in the runtime**; only in tests.

`AppState` currently has `config_tx`, `provider_tx`, `persistence_tx`, `session_store_tx`, `io_tx` as loose `Option` fields. There is no `ActorSystem` or `ActorHandles` struct.

This task introduces the registry in two phases:
1. **Phase A** — create `ActorHandles` for the existing actors, normalize `config_tx`/`provider_tx` to typed handles, and spawn `FffIndexerActor` in the runtime.
2. **Phase B** — add handles as new domain actors (`SessionState`, `Input`, `View`, `Completion`, `Turn`, `Permission`, `Notification`, `Trust`, `Env`, `UiControl`) are implemented.

## Acceptance criteria

- [ ] `ActorHandles` struct exists in `crates/runie-core/src/actors/mod.rs` and holds cloneable handles for existing actors (`config`, `provider`, `persistence`, `session_store`, `io`, `fff_indexer`, `agent`, `ui`, session-log).
- [ ] `AppState` stores one `ActorHandles` instead of loose `Option<Sender>` fields.
- [ ] `config_tx` and `provider_tx` are wrapped in typed handles rather than raw `mpsc::Sender`.
- [ ] `FffIndexerActor` is spawned in the runtime and its handle stored in `ActorHandles`.
- [ ] `ActorHandles` provides `send_*` helpers; tests use a `TestActorHandles` recording variant (see `test-actor-harness`).
- [ ] Actors that need to talk to other actors receive the target handle at construction time (dependency injection), not via `AppState`.
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [ ] `actor_system_send_records_message_in_test_runtime` — test-only runtime captures sent intents.
- [ ] `actor_system_clone_is_shallow` — cloning handles does not duplicate actors.

### Layer 2 — Event Handling
- [ ] `bootstrap_spawns_all_actors` — TUI bootstrap produces a non-empty `ActorSystem`.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/actors/mod.rs` — define `ActorSystem` / `ActorHandles`.
- `crates/runie-core/src/model/state/app_state.rs` — replace loose handles with registry.
- `crates/runie-tui/src/main.rs` / `app_init.rs` — spawn actors and inject registry.
- `crates/runie-core/src/actors/*/actor.rs` — accept downstream handles in constructors where needed.

## Notes

- This task is a prerequisite for all actor-implementation tasks because they all need a handle to send messages through.
- The existing broadcast-subscriber `SessionActor` (durability logger) collides semantically with the planned conversation-state `SessionActor`. Rename the existing one to `SessionLogActor` or keep it as `SessionPersistenceActor`, and use `SessionStateActor` or `ChatSessionActor` for the new owner. Update the registry names accordingly.
- Keep the registry minimal: one field per actor, no generic indirection unless it simplifies the DSL.
