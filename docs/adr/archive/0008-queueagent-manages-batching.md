# QueueAgent owns message queuing and batching

QueueAgent is a dedicated actor managing the message queue and batch delivery. It receives Submit events and decides when to emit SpawnAgent based on queue mode:

- **one-at-a-time**: Each steer triggers a separate LLM call.
- **all**: Messages are batched until the current turn completes, then delivered together.

QueueAgent tracks:
- Pending messages
- Queue mode (per Steering vs FollowUp)
- Batch state

Orchestrator receives the next message from QueueAgent when ready to spawn.
