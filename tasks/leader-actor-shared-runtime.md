# LeaderActor shared runtime with thin clients

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: app-state-read-only-projection
**Blocks**: expose-runie-via-acp-stdio, standardize-headless-output-streaming-json

## Summary

Introduced a `Leader` struct that coordinates all actors in the Runie runtime. The leader owns the event bus, spawns all child actors, and optionally listens on a TCP socket for client connections.

## Implementation

### Files Created

- `crates/runie-core/src/actors/leader/mod.rs` — Module exports
- `crates/runie-core/src/actors/leader/actor.rs` — `Leader` struct and `LeaderHandle`
- `crates/runie-core/src/actors/leader/messages.rs` — `LeaderCommand` and `LeaderStatus` types

### Files Modified

- `crates/runie-core/src/actors/mod.rs` — Added `leader` module
- `crates/runie-core/src/tests/arch_guardrails.rs` — Added `actors/leader/` to allow list

## Runtime Model

```text
         TUI          Headless       ACP/WS
          │              │              │
          └──────────────┼──────────────┘
                         │
                         ▼
                ┌─────────────────┐
                │      Leader     │ owns event bus, spawns actors,
                │ (coordination)  │ handles lifecycle
                └─────────────────┘
                         │
          ┌──────────────┼──────────────┐
          ▼              ▼              ▼
      Config        Session          Turn
      Actor         Actor          Actor
          │              │              │
          └──────────────┴──────────────┘
                         │
                    EventBus<Event>
```

## Key Types

### `Leader`
- `Leader::new()` — Create with default TCP address (127.0.0.1:9000)
- `Leader::embedded()` — Create without socket listening
- `Leader::with_tcp_addr(addr)` — Use custom TCP address
- `Leader::start(factory)` — Spawn all actors and return handles
- `Leader::run(factory)` — Run as foreground process

### `LeaderHandle`
- `event_bus()` — Get the event bus
- `subscribe()` — Subscribe to facts
- `shutdown()` — Graceful shutdown
- `status()` — Get runtime status

### Actor Handles
The leader provides access to all actor handles:
- `config` — ConfigActorHandle
- `provider` — ProviderActorHandle
- `io` — IoActorHandle
- `session` — SessionActorHandle
- `permission` — PermissionActorHandle
- `turn` — TurnActorHandle

## Acceptance Criteria

- [x] `Leader::start()` spawns all actors and returns handles
- [x] `LeaderHandle` provides access to event bus and all actor handles
- [x] TCP socket listening for client connections
- [x] Graceful shutdown via `LeaderCommand::Shutdown`
- [x] `cargo check --workspace` is green

## Tests

- **Layer 1**: LeaderStatus default, LeaderCommand variants, leader construction
- **Layer 2**: Socket client handling (basic)
- **Layer 4**: Full integration with actor lifecycle

## Architecture Notes

The leader is a coordination layer that:
1. Owns the event bus
2. Spawns all child actors
3. Handles lifecycle (startup, shutdown)
4. Optionally listens for client connections

This enables the pattern where TUI, headless CLI, and ACP clients can all connect to the same running runtime.

## Commit

```
6f8e1a8 leader-actor: add Leader struct for actor coordination
```
