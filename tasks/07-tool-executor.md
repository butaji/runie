# Step 07: ToolExecutorActor (sequential + parallel)

**Status:** implemented; provider-specific tool capabilities remain in p37
**Depends on:** 06

## Goal
Implement the tool executor that owns all in-flight tool calls and dispatches them sequentially or in parallel.

## Changes
- `crates/runie-core/src/tools/registry.rs`:
  - `ToolRegistry` holding `HashMap<String, Arc<dyn AgentTool>>`.
  - `lookup(&self, name) -> Option<Arc<dyn AgentTool>>`, `execution_mode(&self, name) -> ToolExecutionMode`.
- `crates/runie-core/src/tools/executor.rs`:
  - `execute_sequential(...)`: loop over tool calls, preflight (validate args via `prepare_arguments`, run `before_tool_call` hook), execute, finalize, emit events.
  - `execute_parallel(...)`: preflight sequentially, execute allowed tools concurrently via `tokio::spawn` owned by a `JoinSet` owned by the caller (passed in), emit `tool_execution_end` in completion order, emit toolResult message events in source order.
- `crates/runie-core/src/tools/actor.rs`:
  - `ToolExecutorActor` with `mpsc::Sender<ToolCommand>`.
  - `ToolCommand::Execute { calls: Vec<ToolCall>, mode: ToolExecutionMode, ctx: ToolExecContext, reply: oneshot::Sender<Vec<ToolOutcome>> }`.
  - Worker holds the registry + event bus handle.
- `crates/runie-core/src/tools/mod.rs`: re-exports + `ToolOutcome` enum (Success { result } | Error { message }).

## Verification
- `cargo check -p runie-core` → exit 0.
- Unit test: register 2 tools (`bash`, `read_file`), execute one call sequentially, assert single `tool_execution_start` + `tool_execution_end` and a toolResult message.
- Unit test: parallel dispatch of 3 calls; verify `tool_execution_end` events arrive in completion order via a logged timestamp ordering check (no `sleep` — use `tokio::task::yield_now()` and `Instant::now()`).

## Notes
- Per-tool `executionMode` override: if ANY call in the batch targets a tool with `executionMode: "sequential"`, the entire batch executes sequentially.
- `before_tool_call` returning `Some(BeforeToolCallResult { block: true, reason })` short-circuits to a synthetic error tool result.
