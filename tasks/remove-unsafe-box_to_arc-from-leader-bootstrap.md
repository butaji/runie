# Remove unsafe `box_to_arc` from leader bootstrap

**Status**: done
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: migrate-tui-and-cli-to-leader-bootstrap
**Blocks**: none

## Description

`crates/runie-core/src/actors/leader/actor.rs` does not contain an unsafe `box_to_arc` helper. The `AgentActorFactoryImpl::spawn` and `spawn_with_join` methods already return `Arc<dyn LeaderAgentHandle>` directly — no reboxing or unsafe conversion is needed.

## Acceptance Criteria

- [x] `AgentActorFactory::spawn` returns `Arc<dyn LeaderAgentHandle>`. (Already the case.)
- [x] `AgentSpawnFuture` resolves to `Arc<dyn LeaderAgentHandle>`. (Already the case.)
- [x] No unsafe `box_to_arc` helper exists in the codebase. (Confirmed absent.)

## Tests

### Layer 1 — State/Logic
- [x] `leader_spawn_returns_arc` — verified by compilation; `spawn_with_join` returns `SpawnedAgent { handle: Arc<dyn LeaderAgentHandle>, ... }`.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `leader_turn_completes_after_arc_change` — all workspace tests pass.

## Files touched

None — no changes needed. The task was already satisfied by earlier actor refactors.

## Notes

- `AgentActorFactoryImpl::spawn` does `Ok(Arc::new(LeaderAgentHandleImpl::new(handle)) as Arc<dyn LeaderAgentHandle>)` — safe and direct.
- `box_to_arc` was described in the task plan but was never present in the code.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
