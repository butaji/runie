# Implement Real AgentRegistry Depth Tracking

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P3

**Depends on**: none
**Blocks**: none

## Description

`AgentRegistry::depth` always returns `0`, so the depth limit check is effectively disabled and `spawn` never rejects for depth.

## Acceptance Criteria

- [ ] Track subagent depth per role or globally.
- [ ] Enforce the ADR depth=1 limit (or configured limit).
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `spawn_rejects_excessive_depth` — depth limit rejects nested spawn.
- [ ] `spawn_allows_within_depth_limit` — valid depth succeeds.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/src/multi_agent.rs`

## Notes

R4 subagent isolation work; safe to defer until subagent pipeline is active.
