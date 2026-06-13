# Runie

Terminal coding agent harness.

## Language

**Event**:
A message emitted to the bus that represents something that happened. Events are immutable and logged for session persistence.
_Avoid_: message (overloaded), signal, notification

**CoreEvent**:
The top-level event type carried by the `EventBus`. Split into durable events (persisted to JSONL) and transient events (UI-only).
_Avoid_: Event (overloaded; legacy name), message

**Durable Event**:
An event that is appended to the session JSONL file and replayed on resume. Examples: `MessageSent`, `ToolCalled`, `ToolResult`, `ModelSwitched`.
_Avoid_: persisted message, log entry

**Transient Event**:
An event that is not persisted. Used for streaming deltas, animation ticks, cursor blinks, and UI phase changes.
_Avoid_: ephemeral event, UI-only message

**EventBus**:
A typed `tokio::sync::broadcast` channel with a bounded replay buffer. All actors publish and subscribe through it.
_Avoid_: channel (too generic), dispatcher

**Actor**:
A concurrent entity with its own event loop, state, and typed input channel. Actors communicate exclusively via the event bus. Runie actors are simple tokio tasks, not a framework runtime.
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
A derived view of state accumulated from events. UIActor projects domain events into view state (Snapshot). SessionActor projects into JSONL. TelemetryAgent projects into usage stats.
_Avoid_: view, derived state, cache

**Batching**:
Collecting user messages that arrive while an agent is processing, then delivering them together. Configurable per Steering vs FollowUp.
_Avoid_: queue (ambiguous — queue holds pending, batch holds for delivery)

**Session**:
A persisted sequence of domain events. Loaded by replaying events into actors. Stored as append-only JSONL.
_Avoid_: history, conversation, context

**Session Store**:
The JSONL persistence layer. Owns file paths, advisory locking, atomic writes, and session replay.
_Avoid_: database, repository

**Skill**:
A self-describing interceptor on the event bus. Subscribes to events, can inject context, modify tool calls, or preprocess input.
_Avoid_: plugin, extension, module

**AgentLoop**:
The pure LLM interaction loop. Receives tool results, calls the LLM, emits `LLMEvent`s. Stateless — no orchestration logic.
_Avoid_: agent (overloaded)

**LLMEvent**:
A provider-agnostic event emitted by the provider layer: `TextDelta`, `ThinkingDelta`, `ToolCallStart`, `ToolCallInputDelta`, `ToolCallEnd`, `Error`, `Usage`, `Finish`.
_Avoid_: ResponseChunk (legacy), provider event

**CommandAgent**:
Actor that parses slash commands and key shortcuts, emits corresponding events to the Orchestrator or other actors.
_Avoid_: command handler, keybinder

**ConfigAgent**:
Actor that owns configuration state. Loads TOML at startup, watches for changes, emits ConfigChanged events.
_Avoid_: settings manager

**ToolRegistry**:
A collection of `Tool` implementations. Built-ins and MCP tools are registered the same way.
_Avoid_: tool manager, tool list

**PermissionSet**:
A list of wildcard rules evaluated last-match to decide `Allow`, `Ask`, or `Deny` for a tool call.
_Avoid_: trust rules, allowlist
