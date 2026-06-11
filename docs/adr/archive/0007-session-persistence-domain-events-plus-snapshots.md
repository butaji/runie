# Session persistence: domain events + periodic snapshots

Sessions are persisted as a sequence of domain events. On load, events are replayed into all actors to reconstruct state.

Snapshots are taken periodically as load accelerators. Loading a session replays from the most recent snapshot, then applies events since then.

The EventBus tags events as ephemeral (not persisted) or domain (persisted).

- Persisted: Submit, SpawnAgent, AgentThinking, AgentResponse, ToolStart, ToolEnd, Done, SwitchModel, etc.
- Not persisted: ScrollUp, CursorLeft, CursorRight, Paste, ToggleExpand, etc.
