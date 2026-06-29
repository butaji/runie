# Remove unsafe `box_to_arc` from leader bootstrap

**Status**: todo
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: migrate-tui-and-cli-to-leader-bootstrap
**Blocks**: none

## Description

`crates/runie-core/src/actors/leader/actor.rs:282-287` contains an unsafe `box_to_arc` helper that reboxes a `Box<dyn LeaderAgentHandle>` into `Arc<dyn LeaderAgentHandle>`. Change `AgentActorFactory::spawn` and `AgentSpawnFuture` to return `Arc<dyn LeaderAgentHandle>` directly and delete the unsafe conversion.

## Acceptance Criteria

- [ ] `AgentActorFactory::spawn` returns `Arc<dyn LeaderAgentHandle>`.
- [ ] `AgentSpawnFuture` resolves to `Arc<dyn LeaderAgentHandle>`.
- [ ] `box_to_arc` is deleted.
- [ ] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [ ] `leader_spawn_returns_arc` — spawned handle is an `Arc`.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `leader_turn_completes_after_arc_change` — provider replay turn still completes.

## Files touched

- `crates/runie-core/src/actors/leader/actor.rs`
- `crates/runie-core/src/actors/leader/mod.rs`

## Notes

- This removes an unnecessary `unsafe` block from the bootstrap path.
