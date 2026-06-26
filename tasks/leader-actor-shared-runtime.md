# LeaderActor shared runtime with thin clients

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: app-state-read-only-projection
**Blocks**: expose-runie-via-acp-stdio, standardize-headless-output-streaming-json

## Summary

Introduce a `LeaderActor` that owns the event bus, session, plan, turn, and MCP state. The TUI, headless CLI, ACP stdio, and WebSocket clients connect as thin producers/consumers of intents and facts.

## Runtime model

```text
         TUI          Headless       ACP/WS
          │              │              │
          └──────────────┼──────────────┘
                         │
                         ▼
                ┌─────────────────┐
                │    LeaderActor  │ owns event bus, session,
                │  (local socket) │ plan, turn, MCP, permissions
                └─────────────────┘
                         │
          ┌──────────────┼──────────────┐
          ▼              ▼              ▼
      Config        Session          Turn
      Actor         Actor            Actor
```

The leader listens on a local socket (default `~/.runie/leader.sock`). Clients send newline-delimited JSON intents; facts stream back. Authentication and permission checks happen inside the leader, not in clients.

## Acceptance Criteria

- `runie leader` starts the shared runtime.
- Clients connect via local socket or stdio and send intents; facts stream back.
- `runie` (TUI) can connect to an existing leader or start a temporary one.
- Headless mode (`runie -p`) can run through the leader.
- Authentication and permission checks are enforced in the leader.
- `cargo check --workspace` is green.

## Tests

- **Layer 2**: Client/leader message flow and lifecycle.
- **Layer 4**: End-to-end turn with a headless client connected to the leader.
