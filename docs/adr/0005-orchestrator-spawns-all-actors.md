# Orchestrator spawns all actors

The Orchestrator is the single spawn point for all actors:
- AgentLoop actors
- ToolActor instances
- SessionManager
- QueueAgent
- ConfigAgent

Cross-cutting concerns that are **stateless or pure** (telemetry, safety,
clipboard, file lookup, command parsing) are **not actors**. They are functions
or modules invoked by the actors above.

The Orchestrator holds typed senders to each actor and routes messages
accordingly. ToolActors are spawned per invocation and send results back via
the bus.

This keeps actor lifecycle centralized and avoids boilerplate for stateless
operations.
