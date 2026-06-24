# EnvActor owns git info and cwd name

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P1

**Depends on**: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection, actor-lifecycle-and-handle-registry
**Blocks**: none

## Description

`git_info` and `cwd_name` are set once during TUI bootstrap via blocking IO (`runie-tui/src/app_init.rs`). They never change after startup but they are still direct state mutations outside any actor. Move the detection into an actor and make the fields event-driven.

Current violators:
- `runie-tui/src/app_init.rs` — sets `state.git_info` and `state.cwd_name` directly (it also loads `skills` and auth providers synchronously; those may be config-derived and already handled by `ConfigActor`).
- `model/state/app_state.rs` — initializes and preserves fields across reset.

## Acceptance criteria

- [ ] `EnvActor` (or reuse `IoActor`) detects cwd and git info asynchronously and emits `Event::EnvDetected { cwd_name, git_info }`.
- [ ] `AppState.git_info` and `cwd_name` are private; reads go through immutable accessors.
- [ ] `runie-tui/src/app_init.rs` sends `EnvMsg::Detect` instead of direct assignment.
- [ ] `AppState::apply_env` (or similar) updates the fields only on `EnvDetected`.
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [ ] `env_actor_detect_emits_env_detected` — detection produces the expected fact.

### Layer 2 — Event Handling
- [ ] `app_init_sends_env_detect` — bootstrap sends the intent.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/actors/env/` — new `mod.rs`, `messages.rs`, `actor.rs` (or add to `actors/io`).
- `crates/runie-core/src/model/state/app_state.rs` — private `git_info`/`cwd_name`.
- `crates/runie-core/src/event/` — add `EnvDetected` variant.
- `crates/runie-tui/src/app_init.rs` — send intent instead of direct write.

## Notes

- If `IoActor` already has a suitable message namespace, prefer adding `EnvMsg` there over a new actor.
