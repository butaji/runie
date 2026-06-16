# Unify Agent Status Enums

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: (none)
**Blocks**: (none)

## Description

Merge three nearly identical status enums into a single unified `AgentLifecycleStatus`.

**Current enums:**
| Enum | Location | States |
|------|----------|--------|
| `TaskStatus` | `orchestrator.rs:113` | Pending, Running, AwaitingUser, Done, Failed |
| `SubagentStatus` | `actors/subagent.rs:126` | Pending, Running, Done{output}, Failed{error} |
| `AgentStatus` | `state.rs:437` | Pending, Running, AwaitingUser, Done, Failed |

**Proposed unified enum:**
```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentLifecycleStatus {
    Pending,
    Running,
    AwaitingUser,
    Done { output: Option<String> },
    Failed { error: String },
}
```

## Acceptance Criteria

- [ ] Unified `AgentLifecycleStatus` enum in `orchestrator.rs`.
- [ ] `TaskStatus` removed, uses `AgentLifecycleStatus` with type alias.
- [ ] `SubagentStatus` removed, uses `AgentLifecycleStatus`.
- [ ] `AgentStatus` in `state.rs` uses `AgentLifecycleStatus`.
- [ ] `From` conversions implemented.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `agent_lifecycle_status_variants` — all variants construct correctly.
- [ ] `agent_lifecycle_status_serialization` — roundtrip works.
- [ ] `from_task_status_converts` — legacy conversion works.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/src/orchestrator.rs`
- `crates/runie-core/src/actors/subagent.rs`
- `crates/runie-core/src/state.rs`
- `crates/runie-core/src/update/mod.rs` (remove mapping boilerplate)
