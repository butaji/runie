# Remove Unused `_plan` Parameter

**Status**: done
**Completed**: 2026-06-16
**Notes**: Removed `_plan` parameter from `SubagentContext::from_task` and updated `SubagentActor::new` and tests. cargo test --workspace passes.
**Milestone**: R4
**Category**: Core / State
**Priority**: P2

**Depends on**: (none)
**Blocks**: (none)

## Description

Remove unused `_plan` parameter from `SubagentContext::from_task`.

**Location:** `crates/runie-core/src/actors/subagent.rs:47`

```rust
pub fn from_task(task: &SubagentTask, _plan: &OrchestratorPlan) -> Self {
//                                                    ^^^^^ unused
```

## Acceptance Criteria

- [ ] `_plan` parameter removed from `from_task` signature.
- [ ] All call sites updated.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
N/A (cosmetic change).

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/src/actors/subagent.rs`

## Notes

Quick fix, 5 minutes.
