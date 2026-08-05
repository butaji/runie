# Step 12: Behavioural tests

**Status:** pending
**Depends on:** 11

## Goal
Cover every behaviour the plan promised in the README parity contract.

## Changes
- `crates/runie-core/tests/event_sequence.rs`:
  - `prompt_hello_event_order`: load `hello.yaml`, run prompt, assert recorded events equal `[AgentStart, TurnStart, MessageStart(user), MessageEnd(user), MessageStart(assistant), MessageUpdate, MessageEnd(assistant), TurnEnd, AgentEnd]`.
- `crates/runie-core/tests/steering.rs`:
  - `steer_mid_run`: `tool_call.yaml` with one bash call that takes 50ms; during the run push a steering message; assert the next turn sees the steering message appended to messages.
- `crates/runie-core/tests/follow_up.rs`:
  - `follow_up_after_completion`: `hello.yaml` (no tools); push a follow-up; assert exactly one more turn runs.
- `crates/runie-core/tests/tool_parallel.rs`:
  - `parallel_completion_order`: `parallel_tools.yaml`; assert `ToolExecutionEnd` events arrive in completion order, `ToolResult` `MessageStart/End` in source order.
- `crates/runie-core/tests/tool_sequential.rs`:
  - `sequential_strict_order`: same fixture, `ToolExecution = Sequential`; assert both event orderings match source order.
- `crates/runie-core/tests/terminate.rs`:
  - `all_terminate_stops`: two tool calls, both set `terminate: true`; assert loop ends after the batch.
  - `mixed_terminate_continues`: two calls, one terminates; assert next turn runs.
- `crates/runie-core/tests/abort.rs`:
  - `abort_during_stream`: `stream_error.yaml` with delay; trigger abort; assert `error_message` set on snapshot, `is_streaming` false, `wait_for_idle` returns promptly.
- `crates/runie-core/tests/hooks.rs`:
  - `before_tool_call_blocks`: register a hook that returns `{ block: true, reason: "test" }`; assert toolResult has error content with reason.
  - `after_tool_call_overrides_content`: hook returns `{ content: vec![TextContent { text: "overridden" }] }`; assert emitted toolResult reflects override.

## Verification
- `cargo test -p runie-core` → all tests pass, exit 0.
- `cargo test -p runie-core --no-run` succeeds.

## Notes
- Each test registers its own `TestLoop` and `TestBus`; no shared global state.
- `tokio::time::pause()` at test start where ordering matters.