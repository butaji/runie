# Actually collapse `ActorHandles` to a typed map

**Status**: done
**Milestone**: R6
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: delete-dead-actor-handle-wrappers
**Blocks**: migrate-tui-and-cli-to-leader-bootstrap

## Description

`ActorHandles` was a custom façade with `Option<Ractor*Handle>` fields and per-actor delegation helpers. This task replaced it with `LeaderHandle` (the canonical typed actor registry from `Leader::start`), making `ActorHandles` a re-export alias.

## What changed

- `ActorHandles` in `crates/runie-core/src/actors/handles.rs` is now a re-export alias: `pub use super::leader::LeaderHandle as ActorHandles`.
- `AppState.actor_handles` type changed from `Option<ActorHandles>` to `Option<LeaderHandle>`.
- TUI `main.rs` removed the `build_actor_handles()` function and `ActorChannels` struct; it now passes `LeaderHandle` directly.
- All downstream code (`domain_ops.rs`, `accessors.rs`, `dsl/runtime.rs`) updated to use `LeaderHandle`.
- `crates/runie-core/src/actors/leader/actor.rs` gained a `test_helpers` module with `test_leader_handle()` for test construction.
- TUI tests updated to use `test_leader_handle()` instead of manually constructing actor fields.
- `update/dialog/open.rs` fixed to handle non-optional `fff_indexer` in `LeaderHandle`.

## Acceptance Criteria

- [x] Replace `AgentHandle` trait (broken dyn) with `AgentHandleBox` enum in `UiActor`.
- [x] Wire `LeaderAgentActorHandle` into TUI bootstrap via `Leader::start`.
- [x] Fix `LeaderAgentHandle::run` to return `Pin<Box<dyn Future>>` for direct `.await`.
- [x] Replace `ActorHandles` with a struct of `ractor::ActorRef<Msg>` fields (one per production actor).
- [x] Delete all delegation helper methods; callers use `actor_ref.cast(...)` or `call!` directly.
- [x] Remove `Option` wrappers where an actor is always present.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `handles_hold_actor_refs` — `ActorHandles` is an alias for `LeaderHandle`, all fields are concrete typed `ActorRef`s.
- [x] `handles_no_delegation_methods` — the old `ActorHandles` struct delegation methods are gone; `LeaderHandle` has no delegation methods.

### Layer 2 — Event Handling
- [x] `leader_config_send_reaches_actor` — config message sent via `LeaderHandle.config` reaches actor.
- [x] `leader_session_send_reaches_actor` — session message via `LeaderHandle.session`.
- [x] `leader_turn_send_reaches_actor` — turn message via `LeaderHandle.turn`.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] Covered by `bootstrap_spawns_all_actors` which verifies all fields of `LeaderHandle` are present.

## Files touched

- `crates/runie-core/src/actors/handles.rs` — now a re-export alias
- `crates/runie-core/src/actors/handles_tests.rs` — updated to test `LeaderHandle`
- `crates/runie-core/src/actors/mod.rs` — re-export comment
- `crates/runie-core/src/actors/leader/actor.rs` — added `test_helpers` module
- `crates/runie-core/src/actors/leader/mod.rs` — re-exports, made `actor` module public
- `crates/runie-core/src/model/state/app_state.rs` — field type
- `crates/runie-core/src/model/state/accessors.rs` — return type
- `crates/runie-core/src/model/state/domain_ops.rs` — uses `LeaderHandle`
- `crates/runie-core/src/dsl/runtime.rs` — uses `LeaderHandle`
- `crates/runie-core/src/update/dialog/open.rs` — fixed non-optional `fff_indexer`
- `crates/runie-tui/src/main.rs` — removed `build_actor_handles()`, `ActorChannels`
- `crates/runie-tui/src/tests/actor_lifecycle.rs` — rewrote to use `Leader::start`
- `crates/runie-tui/src/ui_actor/tests.rs` — uses `test_leader_handle()`

## Notes

- The previous `collapse-actor-handles-to-typed-map.md` task left a façade; this task finishes the job.
- Coordinate with `delete-dead-actor-handle-wrappers.md` — done.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
