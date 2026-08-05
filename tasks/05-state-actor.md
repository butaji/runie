# Step 05: AgentStateActor + snapshot

**Status:** pending
**Depends on:** 04

## Goal
Implement the single-source-of-truth state actor that owns all `AgentState` fields and is the **only** mutator.

## Changes
- `crates/runie-core/src/state/snapshot.rs`:
  - `AgentStateSnapshot` struct (read-only projection): `system_prompt`, `model`, `thinking_level`, `messages`, `tools`, `is_streaming`, `streaming_message`, `pending_tool_calls`, `error_message`.
  - `Clone`, `Debug`, `Default` impls.
- `crates/runie-core/src/state/actor.rs`:
  - `AgentStateActor` struct holding `mpsc::Sender<StateCommand>` and a `watch::Sender<AgentStateSnapshot>`.
  - `StateCommand` enum: `SetSystemPrompt(String)`, `SetModel(Model)`, `SetThinkingLevel(ThinkingLevel)`, `PushMessage(AgentMessage)`, `ReplaceMessages(Vec<AgentMessage>)`, `SetTools(Vec<BoxedAgentTool>)`, `MarkStreaming(bool)`, `SetStreamingMessage(Option<AgentMessage>)`, `AddPendingToolCall(String)`, `RemovePendingToolCall(String)`, `SetError(Option<String>)`, `Reset`.
  - `AgentStateActor::new()` spawns the worker task on the current Tokio runtime; the worker holds the receiver + watch sender + mutable state.
  - Public handle methods wrap each command (`actor.set_system_prompt(s)` → `send(SetSystemPrompt(s)).await`).
  - `subscribe_snapshot(&self) -> watch::Receiver<AgentStateSnapshot>`.
  - `current_snapshot(&self) -> AgentStateSnapshot` (clones from watch).
- `crates/runie-core/src/state/mod.rs`: re-exports.

## Verification
- `cargo check -p runie-core` → exit 0.
- Unit test: spawn actor, push 3 messages, assert snapshot reflects them.
- Unit test: `MarkStreaming(true)` → `is_streaming` true in snapshot.
- Unit test: `Reset` → snapshot back to default.

## Notes
- The worker is the **only** `tokio::spawn` site for this actor; the loop actor never spawns state mutations directly.
- `BoxedAgentTool = Arc<dyn AgentTool<...>>` — tool definitions are shared across snapshot reads.