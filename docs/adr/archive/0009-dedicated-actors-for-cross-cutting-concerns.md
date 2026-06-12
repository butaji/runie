# Cross-cutting concerns: actors vs functions

Only concerns with **async lifecycle or persistent state** become actors.
Everything else is a pure function or module.

| Concern | Form | Rationale |
|---------|------|-----------|
| **ConfigAgent** | Actor | File watcher has async lifecycle |
| **QueueAgent** | Actor | Holds queue state, emits on timer |
| **SessionManager** | Actor | File I/O, async lifecycle |
| **ToolActors** | Actor | Per-invocation async execution |
| **Telemetry** | Function | Stateless accumulator; no async needed |
| **SafetyAgent** | Function | Pure validation; call before bash execution |
| **Clipboard** | Function | One-shot async utility |
| **FileLookup** | Function | One-shot async resolution |
| **Command parsing** | Function | Synchronous parser |

This keeps core logic clean while avoiding actor boilerplate for stateless
operations. Testability is achieved via pure functions (Layer 1) rather than
mock actor topologies.
