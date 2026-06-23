# Actor lifecycle and handle registry

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: actor-owned-state-ssot
**Blocks**: config-ssot-via-configactor, session-actor-owns-session-state, input-actor-owns-input-state, view-actor-owns-view-state, completion-actor-owns-completion-state, turn-actor-owns-agent-turn-state, permission-actor-owns-approvals, notification-actor-owns-transient-messages, trust-actor-owns-trust-decisions, env-actor-owns-git-cwd, fff-indexer-owns-file-picker-results, ui-control-actor-owns-dialog-state

## Description

The new actors need a clear lifecycle: where they are spawned, how their handles reach `AppState`, and how the TUI/bootstrap wires them together. Today `AppState` carries loose `Option<Sender<Msg>>` fields. Replace this with a typed `ActorSystem` or handle registry that is initialized at startup and passed into `AppState`.

## Acceptance criteria

- [ ] A single `ActorSystem` struct (or `ActorHandles`) holds cloneable handles for all actors: `config`, `session`, `input`, `view`, `completion`, `turn`, `permission`, `notification`, `trust`, `env`, `fff_indexer`, `ui_control`.
- [ ] `AppState` stores one `ActorSystem` instead of many `Option<Sender>` fields.
- [ ] `ActorSystem` provides `send_*` helpers that are no-ops when running in a test context without the actor (or tests use a `TestActorSystem` that records messages).
- [ ] TUI bootstrap (`runie-tui/src/main.rs` / `app_init.rs`) spawns all actors in dependency order and injects the registry into `AppState`.
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
- Keep the registry minimal: one field per actor, no generic indirection unless it simplifies the DSL.
