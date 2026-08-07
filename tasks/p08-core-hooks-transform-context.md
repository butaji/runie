# p08 — Core hooks: `transformContext` applied before `convert_to_llm`

**Parity target:** pi `AgentLoopConfig.transformContext`.

## Pi reference

`~/Code/agents/pi/packages/agent/src/agent-loop.ts`
- `streamAssistantResponse` (line 281): `messages = context.messages`; if `config.transformContext`, `messages = await config.transformContext(messages, signal)` (line 289-292). Then `llmMessages = await config.convertToLlm(messages)` (line 295).
- So the transform runs **per turn, before every LLM call**, on the `AgentMessage[]` level (pruning/injection of agent messages, not wire messages).

## Current runie state

`~/Code/GitHub/runie-tests/runie/crates/runie-core/src/loop/driver.rs`
- `run_loop` applies the async loop-owned `transform_context` callback to
  agent messages before the async `convert_to_llm` callback, falling back to
  `default_convert_to_llm` when no converter is supplied.

## Historical implementation plan

1. Add optional `transform_context: Option<Arc<dyn Fn(Vec<AgentMessage>) -> Vec<AgentMessage> + Send + Sync>>` to `RunLoopDeps` (or the `TurnHooks`/`LoopHooks` struct from p07).
2. In `run_loop`, before `default_convert_to_llm`, apply: `let effective = transform_context.map(|f| f(ctx.messages.clone())).unwrap_or(ctx.messages.clone()); let wire = default_convert_to_llm(&effective);`.
3. The transform sees `AgentMessage` (user/assistant/toolResult/custom), runs every turn.

## State machine / variants

Pure transform, no state machine. Variants the transform may return: any subset/reordering of the input `AgentMessage` list. The converted `wire` must be a valid mix of `WireMessage::User|Assistant|ToolResult`.

## Acceptance evidence

- Integration test: a `transform_context` that drops a user message → assert the provider receives a wire context without it; a transform that injects a message → assert it appears.
- `cargo test -p runie-core` green.
## Progress

- **LLM conversion parity (2026-08-05):** Added an async loop-owned
  `convert_to_llm` callback after `transform_context`, with the existing
  converter as the default. Integration coverage proves the callback can
  replace the wire message sent to the provider.

- **Current-state reconciliation (2026-08-08):** the original “no transform”
  statement and three-step adaptation list are historical. `turn_hooks.rs`
  covers both filtering/injection before conversion and converter replacement
  after transformation; `RunLoopDeps` and `LoopDeps` carry both callbacks
  through the actor-owned loop boundary. No implementation work remains in
  this task.
