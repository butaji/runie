# RALPH

- Iteration: 1 of 100
- Prompt file: /Users/admin/.herdr/worktrees/runie/agent-impl/EXECUTE.md
- Updated: 2026-06-08T22:51:08.394Z

## Iteration progression

### Iteration 1

```text
## Summary

I've completed three core architecture tasks:

### 1. ✅ mvp-core-bus (Event Bus)
Created `crates/runie-core/src/event_bus.rs` with:
- `EventTag` enum (Domain vs Ephemeral)
- `DomainEvent` enum (Submit, SpawnAgent, AgentThinking, AgentResponse, etc.)
- `EphemeralEvent` enum (Input, Backspace, CursorLeft/Right, Paste, etc.)
- `BusEventEnvelope` for unified events
- `ActorId` for actor identification
- `ActorChannel<T>` for typed channels
- `SubscriptionFilter` for event filtering
- `BroadcastChannel` for thread-safe event distribution
- 10 unit tests

### 2. ✅ mvp-core-orchestrator (Orchestrator)
Created `crates/runie-core/src/orchestrator.rs` with:
- `ActorHandle` for sending messages to actors
- `Orchestrator` as central spawn point
- Spawn methods: `spawn_agent_loop()`, `spawn_queue_agent()`, `spawn_session_manager()`, `spawn_config_agent()`
- Dynamic `spawn_tool_actor()` for tool actors
- Messaging: `tell()`, `tell_ephemeral()`, `broadcast()`
- 13 unit tests

### 3. ✅ mvp-core-event-unification (Event Unification)
- Removed separate `AgentEvent` type from runie-agent
- runie-agent now uses `runie_core::Event` directly
- Updated runie-term to remove `to_core_event()` conversion
- All **664 tests pass** across all crates

### Commit
``\`
mvp-core: implement EventBus, Orchestrator, and unified events
- Add EventBus with typed channels and event tagging
- Add Orchestrator for spawning all actors
- Unify event types, remove AgentEvent
``\`
```
