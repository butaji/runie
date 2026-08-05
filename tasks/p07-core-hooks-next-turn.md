# p07 — Core hooks: `prepareNextTurn` + `shouldStopAfterTurn`

**Parity target:** pi `AgentLoopConfig` turn hooks.

## Pi reference

`~/Code/agents/pi/packages/agent/src/agent-loop.ts`
- `prepareNextTurn?` (line 232): called after `emit turn_end`, before steering poll. Input `PrepareNextTurnContext = {message, toolResults, context, newMessages}` (types.ts:142). Returns `AgentLoopTurnUpdate = {context?, model?, thinkingLevel?}` (types.ts:133). If returned, the loop replaces `currentContext`, `config.model`, and `config.reasoning` (line 233-245). `thinkingLevel` maps: `undefined`→keep, `"off"`→undefined, else the level.
- `shouldStopAfterTurn?` (line 247): called with `ShouldStopAfterTurnContext = {message, toolResults, context, newMessages}` (types.ts:121). If truthy → emit `agent_end`, return immediately (before steering/follow-up poll).

## Current runie state

`~/Code/GitHub/runie-tests/runie/crates/runie-core/src/loop/driver.rs` + `r#loop/turn.rs`
- `decide_next_turn` (turn.rs:18) is a pure function covering only: pending tool calls → ToolBatch; non-empty queues → Continue; state error → Stop(error); else Stop(no-more-work).
- No `prepareNextTurn` or `shouldStopAfterTurn` equivalent.

## Adapt to runie

Add to `RunLoopDeps` two optional hooks (`Arc<dyn Fn(...) + Send + Sync>`) or a `LoopHooks` struct:

```rust
pub struct TurnHooks {
    pub prepare_next_turn: Option<Arc<dyn Fn(PrepareNextTurnContext)
        -> BoxFuture<'static, Option<TurnUpdate>> + Send + Sync>>,
    pub should_stop_after_turn: Option<Arc<dyn Fn(ShouldStopAfterTurnContext) -> bool + Send + Sync>>,
}
pub struct TurnUpdate { pub context: Option<AgentContext>, pub model: Option<Model>, pub thinking_level: Option<ThinkingLevel> }
```

In `run_loop` after `TurnEnd`, before the steering drain:
1. If `should_stop_after_turn` returns true → publish `AgentEnd`, return.
2. If `prepare_next_turn` returns `Some(update)` → replace `deps`-level context/model/thinking for the next turn (the loop's `ctx`/`model` variables, driver.rs ~line 71-75).

## State machine / variants

Per-turn hook sequence:
```
... stream → turn_end →
  [shouldStopAfterTurn? -> true → agent_end STOP]
  [prepareNextTurn? -> update context/model/thinking]
  → drain steering/follow-up → turn_start (next) | stop
```

## Acceptance

- Integration test: a `prepare_next_turn` that swaps the model → assert the second provider call receives the new model; a `should_stop_after_turn` that returns true after turn 1 → assert `agent_end` fires with no second turn.
- `cargo test -p runie-core` green.