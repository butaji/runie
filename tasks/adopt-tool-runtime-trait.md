# Adopt Unified Tool Runtime Trait

**Status**: todo
**Milestone**: R4
**Category**: Tools
**Priority**: P1

**Depends on**: (none)
**Blocks**: (none)

## Description

Define a unified `ToolRuntime` trait that provides a consistent interface for all tool execution:

```rust
pub trait ToolRuntime<Rq, Out>: Send + Sync {
    fn exec_approval_requirement(&self, req: &Rq) -> Option<ExecApprovalRequirement>;
    fn network_approval_spec(&self, req: &Rq, ctx: &ToolCtx) -> Option<NetworkApprovalSpec>;
    async fn run(&mut self, req: &Rq, attempt: &SandboxAttempt, ctx: &ToolCtx) 
        -> Result<Out, ToolError>;
}
```

Reference: `~/Code/agents/codex-rs/tools/src/tool_runtime.rs`

Benefits:
- Type-safe tool implementations with compile-time guarantees
- Consistent approval/sandboxing behavior
- Pluggable execution strategies

## Acceptance Criteria

- [ ] `ToolRuntime` trait defined in `runie-core/src/tool/runtime.rs`.
- [ ] Built-in tools (Read, Write, Edit, Bash, Grep) implement `ToolRuntime`.
- [ ] MCP tools wrap underlying runtime via `ToolRuntime`.
- [ ] Tool orchestrator uses trait for generic execution.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `tool_runtime_exec_approval_requirement` — returns correct approval level.
- [ ] `tool_runtime_run_returns_output` — successful execution returns result.
- [ ] `tool_runtime_run_returns_error` — failed execution returns error.

### Layer 2 — Event Handling
- [ ] `tool_execution_emits_events` — start/complete/error events emitted.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
- [ ] Smoke test executes all built-in tools.

## Files touched

- `crates/runie-core/src/tool/runtime.rs` (new)
- `crates/runie-core/src/tool/mod.rs`
- Built-in tool implementations

## Notes

This enables future sandbox strategies (Landlock, Seccomp) to plug into the runtime.
