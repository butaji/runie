# Runie

Terminal coding agent harness.

## Language

**Event**:
A message emitted to the bus that represents something that happened. Events are immutable and logged for session persistence.
_Avoid_: message (overloaded), signal, notification

**Actor**:
A concurrent entity with its own event loop, state, and typed input channel. Actors communicate exclusively via the event bus.
_Avoid_: agent (overloaded), worker, task

**Orchestrator**:
Central actor that manages agent lifecycle — spawning agents, tracking completion, and routing tool results. Designed for future sequential and parallel flow orchestration.
_Avoid_: coordinator, manager, supervisor

**ToolActor**:
An actor representing a single tool invocation. Self-describes via ToolRegistered, executes asynchronously, emits ToolEnd with results.
_Avoid_: tool instance, worker, executor

**Snapshot**:
Immutable frame description — the UI's view of the world at a point in time. Produced by UIAgent as a projection from the event stream.
_Avoid_: frame, state dump

**Projection**:
A derived view of state accumulated from events. UIAgent projects domain events into view state (Snapshot). TelemetryAgent projects into usage stats.
_Avoid_: view, derived state, cache

**Batching**:
Collecting user messages that arrive while an agent is processing, then delivering them together. Configurable per Steering vs FollowUp.
_Avoid_: queue (ambiguous — queue holds pending, batch holds for delivery)

**Session**:
A persisted sequence of domain events. Loaded by replaying events into actors. Augmented with periodic snapshots as load accelerators.
_Avoid_: history, conversation, context

**Skill**:
A self-describing interceptor on the event bus. Subscribes to events, can inject context, modify tool calls, or preprocess input.
_Avoid_: plugin, extension, module

**AgentLoop**:
The pure LLM interaction loop. Receives tool results, calls the LLM, emits response chunks. Stateless — no orchestration logic.
_Avoid_: agent (overloaded)

**CommandAgent**:
Actor that parses slash commands and key shortcuts, emits corresponding events to the Orchestrator or other actors.
_Avoid_: command handler, keybinder

**ConfigAgent**:
Actor that owns configuration state. Loads TOML at startup, watches for changes, emits ConfigChanged events.
_Avoid_: settings manager
