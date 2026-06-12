# ToolActors self-describe via events

Each ToolActor, on spawn, emits a `ToolRegistered { name, description, schema }` event. AgentLoop subscribes to these events on startup and builds its tool list dynamically.

Tools are executed asynchronously. ToolActor receives `ToolStart`, executes the tool, emits `ToolEnd { output }`.

Orchestrator spawns ToolActors and forwards tool results to AgentLoop.

This makes the system extensible without modifying the agent or orchestrator code.
