# p10 — Core tools: beforeToolCall block, terminate AND semantics, parallel completion-order events

## Pending Pi event-vector migration (2026-08-06)

The current executor still emits a synthetic `tool_execution_update` with
`{"status":"running"}` before dispatch. Pi emits updates only from a tool's
`onUpdate` callback. Removing this compatibility event requires regenerating
the 61 declarative trace sidecars that currently include it; keep this
explicitly tracked rather than silently weakening exact event assertions.

**Latest parity note (2026-08-05):** `afterToolCall` overrides now propagate
`content`, `details`, `usage`, `terminate`, and `isError` into both
`tool_execution_end` and the resulting `toolResult` message.

The callback also receives the actor-owned current `AgentContext`, matching
pi's `AfterToolCallContext` contract.

**Parity target:** pi tool dispatch (`executeToolCalls` family).

## Pi reference

`~/Code/agents/pi/packages/agent/src/agent-loop.ts`
- Dispatch mode (line 411): sequential if `config.toolExecution === "sequential"` **or any** tool has `executionMode === "sequential"`; else parallel.
- `prepareToolCall` (line 600): missing tool → immediate error `"Tool <name> not found"`; `prepareToolCallArguments`; `validateToolArguments`; `beforeToolCall` hook — if `signal.aborted` → `"Operation aborted"`; if `beforeResult?.block` → `createErrorToolResult(beforeResult.reason || "Tool execution was blocked")`; success → `{kind:"prepared", ...}`.
- `executePreparedToolCall` (line 666): `acceptingUpdates=true`; `tool.execute(id, args, signal, onUpdate)`; `onUpdate` pushes `tool_execution_update`; after settle return `{result, isError:false}`; on throw → `createErrorToolResult(msg)`, `isError:true`.
- `finalizeExecutedToolCall` (line 709): `afterToolCall` hook merges field-by-field: `content??result.content`, `details??result.details`, `usage??result.usage`, `terminate??result.terminate`, `isError = afterResult.isError ?? isError`.
- `shouldTerminateToolBatch` (line 582): `terminate === true` on **every** finalized result (AND).
- Parallel (line 489): emit all `tool_execution_start` + `prepareToolCall` up front; `tool_execution_end` fires in **completion order** (inside closures); then tool-result messages emitted as `message_start`/`message_end` **in assistant source order** (line 540-548).
- `createErrorToolResult` (line 756): `{content:[{type:"text",text}], details:{}}`.
- `createToolResultMessage` (line 773): `content: result.content ?? []`, `details`, `usage`, `addedToolNames?.length ? {addedToolNames} : {}`, `isError`, `timestamp`.

## Current runie state

`~/Code/GitHub/runie-tests/runie/crates/runie-core/src/tools/executor.rs`
- `execute_sequential` (line 50) / `execute_parallel` (line 100) exist. `BeforeToolCallResult.block` exists (types.rs:437). `all_terminated` computed (executor.rs:87). `synthetic_error_result` (executor.rs:253).

## Adapt to runie (verify + close gaps)

1. **beforeToolCall block** — confirm the executor honors `BeforeToolCallResult.block` → synthetic error with `reason || "Tool execution was blocked"`.
2. **terminate AND semantics** — confirm `all_terminated` is true only when every result `terminate === true` (executor.rs:87-95 already implements the AND; verify alertness).
3. **parallel completion-order**: `tool_execution_end` events must fire in completion order while `message_start`/`end` for tool results fire in **source order**. Verify `execute_parallel` separates these two orders (see comment at executor.rs:5).
4. **dispatch mode detection**: parallel vs sequential must honor both the global `tool_execution` and any per-tool `execution_mode()==Sequential` (types.rs:349). Verify the driver/executor checks per-tool.
5. **Abort mid-batch**: sequential/parallel honor the cancellation token and break on abort (pi `agent-loop.ts:433,489`).

## State machine / variants

Per-call lifecycle:
```
start → prepare (missing tool | prepareArgs | validate | beforeToolCall)
      ├── immediate-error (missing/blocked/aborted/validation)
      └── prepared → execute (on_update*) → finalize (afterToolCall merge)
                    → success | error
```
Batch terminate = AND over every finalized result.terminate.
Result order: parallel completion order for events; source order for messages.

## Acceptance

- New tests: block via `before_tool_call` yields the exact pi error text; `terminate` AND semantics (one non-terminated result → batch not terminated); parallel completion-order != source-order when tools finish out of order; sequential-vs-parallel dispatch selection incl. per-tool override; abort mid-batch stops remaining calls.
- `cargo test -p runie-core` green.

## Progress

- **Hook context parity (2026-08-05):** `before_tool_call` and
  `after_tool_call` both receive the actor-owned `AgentContext`; the dispatch
  regression verifies the two callbacks observe the current two-message
  transcript for both parallel tool calls.
- **Async hook parity (2026-08-05):** Both tool-hook callbacks now return
  boxed futures and are awaited inside the async executor, matching pi's
  asynchronous hook contract without blocking the actor worker.
- **Tool update parity (2026-08-05):** The executor now supplies each tool
  with an `on_update` callback and projects partial results as
  `ToolExecutionUpdate` events through the existing event sequence.
- **Tool cancellation parity (2026-08-05):** The loop abort receiver now
  crosses the tool actor boundary; each tool receives a
  `CancellationToken`, and execution races the tool future against abort so
  cancellation uses the same synthetic error lifecycle as other tool errors.
  A focused executor regression now holds a tool in flight, aborts it, and
  asserts the typed error result.
- **Hook signal parity (2026-08-05):** Before/after tool-hook payloads now
  expose the per-call `CancellationToken`; the hook regression confirms the
  signal is live while both callbacks execute.
- **Live update parity (2026-08-05):** Tool `on_update` callbacks now publish
  `ToolExecutionUpdate` directly through the shared event bus from the owning
  tool actor; the loop skips replaying those already-published updates while
  retaining them in the batch oracle.
- **Failure finalization parity (2026-08-05):** Execution errors and aborts
  now pass through `after_tool_call` with `is_error=true`; hook overrides are
  applied before the final tool event/result is emitted.

- **Tool hook wire parity (2026-08-05):** Tool execution results and
  before/after hook payloads now serialize pi-compatible camelCase keys,
  including `addedToolNames` and `isError`, with focused serialization
  coverage.
## Completed Pi event-vector migration (2026-08-06)
Removed the synthetic running update from parallel tool dispatch. `ToolExecutionUpdate`
now represents only a tool's callback-driven partial result, matching Pi's
`tool_execution_update` contract. Updated all affected YAML exact-event oracles;
`replay_provider` passes across the complete trace corpus.
