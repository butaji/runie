# Adopt Unified Tool Runtime Trait

**Status**: done
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

- [x] `ToolRuntime` trait defined in `runie-core/src/tool/runtime.rs`.
- [x] Built-in tools (Read, Write, Edit, Bash, Grep) implement `ToolRuntime` via a single impl on the `runie-agent::tools::Tool` enum.
- [ ] MCP tools wrap underlying runtime via `ToolRuntime`. (deferred — current MCP module is config/status only; no MCP tool execution runtime exists yet)
- [~] Tool orchestrator uses trait for generic execution. (foundational — built-in `Tool` enum implements `ToolRuntime`; the existing `ToolRegistry`/`Tool` orchestrator path remains unchanged to avoid a larger refactor)
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `tool_runtime_exec_approval_requirement` — returns correct approval level.
- [x] `tool_runtime_run_returns_output` — successful execution returns result.
- [x] `tool_runtime_run_returns_error` — failed execution returns error.

### Layer 2 — Event Handling
- [ ] `tool_execution_emits_events` — start/complete/error events emitted. (deferred; existing orchestrator event emission unchanged)

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
- [ ] Smoke test executes all built-in tools. (deferred; existing tool unit tests cover Read/Bash/Write/Edit/Grep/Find)

## Test Results

```text
$ cargo test -p runie-core
running 1529 tests
test result: ok. 1529 passed; 0 failed; 1 ignored

$ cargo clippy -p runie-core -- -D warnings
    Finished `dev` profile [unoptimized] target(s) in 0.14s

$ cargo test --workspace
(all crates) test result: ok

$ cargo clippy --workspace -- -D warnings
    Finished `dev` profile [unoptimized] target(s) in 3.13s
```

## Files touched

- `crates/runie-core/src/tool/runtime.rs` (new)
- `crates/runie-core/src/tool/mod.rs`
- `crates/runie-agent/src/tools/runtime.rs` (new)
- `crates/runie-agent/src/tools/mod.rs`
- `crates/runie-agent/Cargo.toml`

## Design Decisions

- Simplified the trait from the original generic `ToolRuntime<Rq, Out>` sketch to a concrete, stateful trait: `run(&self, ctx: &ToolContext) -> Result<ToolOutput, ToolError>`. This matches the existing codebase where the agent `Tool` enum already carries its request arguments.
- `ToolRuntime` is implemented once for the whole built-in `Tool` enum in `runie-agent`, covering Read, Write, Edit, Bash, Grep, Find, and FetchDocs without duplicating logic.
- Approval/sandbox types (`ExecApprovalRequirement`, `NetworkApprovalSpec`, `SandboxAttempt`, `ToolError`) are defined in `runtime.rs` and re-exported from `runie_core::tool`.
- `ToolError` uses manual `Display`/`Error` impls to avoid adding a new `thiserror` dependency to `runie-core`.
- The existing orchestrator still dispatches through `runie_core::tool::ToolRegistry` (`Arc<dyn Tool>`). Migrating it fully to `ToolRuntime` would require either a wrapper factory or replacing the registry, which is out of scope for this foundational trait PR.

## Notes

This enables future sandbox strategies (Landlock, Seccomp) to plug into the runtime.
