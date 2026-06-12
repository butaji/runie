# Actor-based architecture with shared event bus

All concurrency is structured as actors communicating via a shared event bus. Each actor has its own event loop, typed input channel, and accumulated state. No shared mutable state between actors.

The event bus is the central hub: actors publish events to it and subscribe to receive events they care about. The bus tags events as ephemeral (not persisted) or domain (persisted).

This replaces the previous model where a single `AppState` was mutated by event handlers.
