# Orchestrator spawns all actors

The Orchestrator is the single spawn point for all actors:
- AgentLoop actors
- ToolActor instances
- SessionManager
- QueueAgent
- ConfigAgent
- TelemetryAgent
- SafetyAgent
- ClipboardAgent
- FileLookupActor
- CommandAgent

The Orchestrator holds typed senders to each actor and routes messages accordingly. ToolActors are spawned per invocation and send results back via the bus.

This keeps actor lifecycle centralized and enables future sequential/parallel flow management.
