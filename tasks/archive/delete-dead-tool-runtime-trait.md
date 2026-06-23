# Delete dead duplicate ToolRuntime trait

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

Two `pub trait ToolRuntime` coexist in `runie-core`. The live one at `tool_runtime.rs:43` (`execute(ToolCall) -> Result<ToolResult, ToolError>`) is used by `EngineToolRuntime` and agent turns. The dead one at `tool/runtime.rs:92` (rich trait with `name()`, `exec_approval_requirement()`, `network_approval_spec()`, `run(&ToolContext)`) has zero external importers — only its own `#[cfg(test)] mod tests` consumes it. Its exported types (`ExecApprovalRequirement`, `NetworkApprovalSpec`, `SandboxAttempt`, rich `ToolError`) are re-exported from `tool/mod.rs:21-23` but never imported anywhere. 183 lines of abandoned sandbox-design scaffolding.

## Acceptance Criteria

- [ ] `crates/runie-core/src/tool/runtime.rs` deleted.
- [ ] Re-exports of `ToolRuntime`, `ExecApprovalRequirement`, `NetworkApprovalSpec`, `SandboxAttempt`, `ToolError` removed from `tool/mod.rs`.
- [ ] Only one `trait ToolRuntime` remains in the workspace (`tool_runtime.rs`).
- [ ] `rg "tool::runtime|tool::ExecApprovalRequirement|tool::NetworkApprovalSpec|tool::SandboxAttempt" crates/` returns zero hits.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `only_one_toolruntime_trait_exists` — grep assertion: exactly one `pub trait ToolRuntime` definition in `crates/`.

### Layer 2 — Event Handling
- N/A — trait cleanup, not event flow.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_agent_turn_uses_live_toolruntime` — a mock-tool-runtime turn still completes after deletion.

## Files touched

- `crates/runie-core/src/tool/runtime.rs`
- `crates/runie-core/src/tool/mod.rs`

## Notes

If the sandbox/approval design is wanted later, reintroduce it as an extension of the live trait rather than a parallel hierarchy. Do not keep dead scaffolding "for later".
