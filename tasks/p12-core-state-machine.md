# p12 — Core state: `AgentState` computed fields + transition state machine

**Parity target:** pi `AgentState` (read-only projection over the actor).

## Pi reference

`~/Code/agents/pi/packages/agent/src/types.ts:327`
```ts
AgentState {
  systemPrompt: string; model: Model; thinkingLevel: ThinkingLevel;
  tools: AgentTool[]; messages: AgentMessage[];
  readonly isStreaming: boolean;
  readonly streamingMessage?: AgentMessage;
  readonly pendingToolCalls: ReadonlySet<string>;
  readonly errorMessage?: string;
}
```
- `isStreaming` / `streamingMessage` / `pendingToolCalls` / `errorMessage` are **computed projections** from the mutable core fields, not stored directly.
- The `Agent` facade keeps `isStreaming=true` until `agent_end` settles
  (`types.ts:343,418`); `message_end(assistant)` closes the message but is not
  the run-settlement boundary. `streamingMessage` is the live partial;
  `pendingToolCalls` are the currently executing ids; `errorMessage` is set on
  failure.

## Current runie state

`~/Code/GitHub/runie-tests/runie/crates/runie-core/src/state/`
- `AgentStateActor` (actor.rs) + `AgentStateSnapshot` (snapshot.rs). `AgentState` (types.rs:325) has `system_prompt, model, thinking_level, messages, tools`; `mark_streaming`, `set_error`, `push_message` exist (used in driver.rs).

## Adapt to runie

1. Add computed projection methods to the snapshot/state:
   - `is_streaming(): bool` — true from assistant `message_start` through
     assistant `message_end`, until `agent_end` (or an explicit error).
   - `streaming_message(): Option<&AssistantMessage>` — the in-progress assistant message.
   - `pending_tool_calls(): Vec<String>` — tool call ids in flight (maintained via a set updated at `ToolExecutionStart`/`End`).
   - `error_message(): Option<&str>`.
2. These must be **rebuilt from events** (single-source-of-truth principle): the actor records the stream markers and tool ids from the events it observes, and the getters are pure projections over that state.
3. Ensure the actor's transitions fire on the right events: `mark_streaming(true/false)` (already), set/clear `pending_tool_calls` on tool start/end, set `error_message` on `Error`/abort.

## State machine / variants

`AgentStateActor` phases (projection):
```
idle --message_start(assistant)--> streaming(streaming_message=live)
streaming --message_end(assistant)--> settling
settling --agent_end--> idle
idle --tool_execution_start--> running_tool(pending_tool_calls += id)
running_tool --tool_execution_end--> idle (pending_tool_calls -= id)
any --error/abort--> errored(error_message=Some) --agent_end--> idle
```
Variants: `is_streaming ∈ {true,false}`; `pending_tool_calls ⊆ {active tool ids}`; `error_message ∈ {None} ∪ {Some(reason)}`.

## Acceptance

- Unit tests: projection getters reflect event application (stream open/close, tool start/end, error set); rebuild-from-events equivalence (replaying the same events produces the same snapshot).
- `cargo test -p runie-core` green.
## Progress

- **Projection synchronization (2026-08-05):** in-flight streaming and
  pending-tool projection tests now await the state actor's synchronization
  contract before asserting terminal state, eliminating scheduler-dependent
  reads while preserving actor-only mutation.

- **Agent settlement boundary (2026-08-06):** the reducer now matches Pi's
  documented lifecycle: assistant `message_end` records the final message but
  leaves `is_streaming` true; `agent_end` clears streaming after turn hooks and
  queue decisions have settled.

- **Run-start projection (2026-08-06):** `agent_start` now crosses the same
  actor/event boundary as the rest of the live loop. It reopens streaming,
  clears the previous error and pending-call projection, and lets a new run
  begin from a clean runtime state, matching `Agent.runWithLifecycle`.

- **Settlement cleanup (2026-08-06):** `agent_end` now clears pending tool
  calls as well as streaming state. This matches Pi's `finishRun()` behavior
  for aborted or otherwise interrupted tool batches.

- **Error timing (2026-08-06):** error projection now follows Pi's reducer
  ordering: `message_end` stores the assistant message, while `turn_end`
  promotes its `error_message` into `AgentStateSnapshot.error_message`.
  Both live driver `turn_end` branches now use the actor event boundary, so
  abort/error state is visible in the authoritative snapshot.
# Latest parity correction (2026-08-06)

The tool registry is now projected into `AgentStateSnapshot.tools` at the
actor-owned loop boundary before each prompt or continuation. This matches
Pi's agent state, where registered tools are part of the context contract
rather than only an executor-side lookup table. YAML `tool_count` and
`tool-echo.yaml` cover the event-sequence-to-state path.

Caller-supplied non-empty `AgentContext` now also enters the state actor before
the loop starts: system prompt and prior messages are projected through actor
commands, while an empty context preserves the existing run state. A turn-hook
regression verifies the provider context and actor snapshot agree.
