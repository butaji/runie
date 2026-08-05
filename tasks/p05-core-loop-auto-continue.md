# p05 — Core loop: auto-continue after tool batches

**Parity target:** pi-agent-core inner-loop continuation.

## Pi reference

`~/Code/agents/pi/packages/agent/src/agent-loop.ts`
- Inner loop `while (hasMoreToolCalls || pendingMessages.length > 0)` (line 174).
- After dispatching tool calls: `hasMoreToolCalls = !executedToolBatch.terminate` (line 216).
- `terminate` is **true only when every finalized result** sets `result.terminate === true` — `shouldTerminateToolBatch`, line 582.
- The truncated path returns `terminate: false` — `failToolCallsFromTruncatedMessage`, line 405.
- When `hasMoreToolCalls` is true, the loop streams the **next** assistant response (which consumes the tool results) in the same inner iteration, emitting `turn_start`.
- `turn_start` is emitted once per inner iteration **except the first** (guarded by `firstTurn`, line 175-179).

## Current runie state

`~/Code/GitHub/runie-tests/runie/crates/runie-core/src/loop/driver.rs`
- After a `TurnPlan::ToolBatch`, the loop drains steering/follow-up and **breaks** if none were injected (`if !any_injected { break; }`). It does **not** auto-continue to consume tool results.
- `ToolOutcome::Completed` already carries `all_terminated: bool` (`tools/actor.rs:17`).

## Adapt to runie

1. Extend `ToolExecOutcome` (driver.rs) with `all_terminated: bool`.
2. In the `ToolBatch` branch, compute `has_more_tool_calls`:
   - normal path: `!all_terminated` (from `ToolOutcome::Completed`).
   - truncated path (`fail_truncated_calls`): `true` (pi returns `terminate:false`).
   - guard against infinite replay: `has_more_tool_calls = !tool_results.is_empty() && !all_terminated` (empty preflighted batch must not loop).
3. Change the loop-termination condition to `if !any_injected && !has_more_tool_calls { break; }`, and publish `TurnStart` before the next iteration (already at the end of the loop body).
4. The next provider call includes the tool results because `deps.state.push_message(tool_result)` already ran (driver.rs ~`push_message`).

## State machine / variants

Loop continuation state:
```
idle/prompt → turn_start → stream assistant
  ├─ done(error|aborted) → turn_end → agent_end (STOP)
  ├─ toolUse → execute batch → has_more = !terminate
  │    ├─ has_more=true → turn_end → turn_start → stream next (CONTINUE)
  │    └─ has_more=false → turn_end → drain steering/follow-up → maybe turn_start → else STOP
  └─ stop → turn_end → drain steering/follow-up → maybe continue else STOP
```
`terminate` variants: a tool result with `terminate:true` ends the batch only if **every** result in the batch is terminated (AND).

## Acceptance

- New integration test: a stream whose first turn yields a tool call (`Done{tool_use}`) and whose second turn yields `Done{stop}` — assert the loop produces **two** assistant messages and `TurnStart` appears twice (one per turn).
- Update the truncated-tool test (p02 guard) to the two-turn shape.
- Existing tool fixtures drive the replay provider to return a terminating Stop ack on the second call (see p11).
- `cargo test -p runie-core` green.