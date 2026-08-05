# p08 — Core hooks: `transformContext` applied before `convert_to_llm`

**Parity target:** pi `AgentLoopConfig.transformContext`.

## Pi reference

`~/Code/agents/pi/packages/agent/src/agent-loop.ts`
- `streamAssistantResponse` (line 281): `messages = context.messages`; if `config.transformContext`, `messages = await config.transformContext(messages, signal)` (line 289-292). Then `llmMessages = await config.convertToLlm(messages)` (line 295).
- So the transform runs **per turn, before every LLM call**, on the `AgentMessage[]` level (pruning/injection of agent messages, not wire messages).

## Current runie state

`~/Code/GitHub/runie-tests/runie/crates/runie-core/src/loop/driver.rs`
- `run_loop` builds `wire` via `default_convert_to_llm(&ctx.messages)` (driver.rs ~line 78) with no transform step.

## Adapt to runie

1. Add optional `transform_context: Option<Arc<dyn Fn(Vec<AgentMessage>) -> Vec<AgentMessage> + Send + Sync>>` to `RunLoopDeps` (or the `TurnHooks`/`LoopHooks` struct from p07).
2. In `run_loop`, before `default_convert_to_llm`, apply: `let effective = transform_context.map(|f| f(ctx.messages.clone())).unwrap_or(ctx.messages.clone()); let wire = default_convert_to_llm(&effective);`.
3. The transform sees `AgentMessage` (user/assistant/toolResult/custom), runs every turn.

## State machine / variants

Pure transform, no state machine. Variants the transform may return: any subset/reordering of the input `AgentMessage` list. The converted `wire` must be a valid mix of `WireMessage::User|Assistant|ToolResult`.

## Acceptance

- Integration test: a `transform_context` that drops a user message → assert the provider receives a wire context without it; a transform that injects a message → assert it appears.
- `cargo test -p runie-core` green.