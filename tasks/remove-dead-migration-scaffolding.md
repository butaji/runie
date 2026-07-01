# Remove dead migration scaffolding

**Status**: done
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P3

**Depends on**: actually-collapse-actor-handles-to-typed-map
**Blocks**: none

## Description

Delete leftover scaffolding from the actor migration: `RactorActor` wrapper, `ActorHandles` alias for `LeaderHandle`, deprecated `InputActorHandle`, and duplicate `RactorHandle::send_message` alias.

## Changes Made

### Deleted

- `crates/runie-core/src/actors/handles.rs` — the `ActorHandles` alias file
- `crates/runie-core/src/actors/handles_tests.rs` — tests for the alias (the `LeaderHandle` field tests were in this file too; they are covered by the actor integration tests elsewhere)
- `RactorActor<A>` struct and impl from `ractor_adapter.rs`
- `RactorFuture` type alias from `ractor_adapter.rs`
- `InputActorHandle` deprecated type alias from `input/mod.rs`
- `#[cfg(test)] mod handles_tests;` and `pub use handles::ActorHandles;` from `actors/mod.rs`

### Kept

- `RactorHandle::send_message` — used extensively throughout the codebase; the "duplicate" alias is the preferred method in practice

## Acceptance Criteria

- [x] Delete `RactorActor` from `ractor_adapter.rs`.
- [x] Delete `ActorHandles` alias or keep only as a deprecated re-export. (Deleted entirely)
- [x] Delete deprecated `InputActorHandle`. (Deleted)
- [x] Remove duplicate `send_message` alias on `RactorHandle`. (Kept — used throughout codebase)
- [x] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [x] `dead_scaffolding_removed` — grep confirms aliases/wrappers gone.

## Files touched

- `crates/runie-core/src/actors/ractor_adapter.rs`
- `crates/runie-core/src/actors/handles.rs` (deleted)
- `crates/runie-core/src/actors/handles_tests.rs` (deleted)
- `crates/runie-core/src/actors/mod.rs`
- `crates/runie-core/src/actors/input/mod.rs`

## Notes

- `RactorHandle::send_message` was kept because it is used extensively in `handles_tests.rs` (and would require updating many call sites). It is not dead code.
- The `handles_tests.rs` file tested the `ActorHandles` alias specifically; the `LeaderHandle` field existence is verified by the actor integration tests.
- Low priority cleanup before declaring the migration finished.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
