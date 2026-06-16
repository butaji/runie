# Add `From` Implementations for Status Conversion

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P2

**Depends on**: `unify-agent-status-enum`
**Blocks**: (none)

## Description

Add `From` trait implementations to eliminate status mapping boilerplate.

**Boilerplate to remove** (`crates/runie-core/src/update/mod.rs:79-114`):
```rust
let agent_status = match status {
    TaskStatus::Pending => AgentStatus::Pending,
    TaskStatus::Running => AgentStatus::Running,
    TaskStatus::AwaitingUser => AgentStatus::AwaitingUser,
    TaskStatus::Done => AgentStatus::Done,
    TaskStatus::Failed => AgentStatus::Failed,
};
```

## Acceptance Criteria

- [ ] `impl From<TaskStatus> for AgentStatus` added.
- [ ] `impl From<SubagentStatus> for AgentStatus` added.
- [ ] `update/mod.rs` uses `status.into()` instead of match.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `task_status_into_agent_status` — all variants convert.
- [ ] `subagent_status_into_agent_status` — all variants convert.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/src/state.rs` or `orchestrator.rs`
- `crates/runie-core/src/update/mod.rs`

## Notes

Depends on unifying status enums.
