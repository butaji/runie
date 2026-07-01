# Remove direct `AppState` mutation from core update handlers

## Status

`todo`

## Description

`AppState` is a read-only UI projection of actor-owned state. Several core update handlers still mutate `AgentState`, session messages, and turn state directly instead of routing changes through the owning actor and reacting to emitted events.

Target locations:
- `crates/runie-core/src/update/agent/core_messages.rs:70,96,176-184,271-286`
- `crates/runie-core/src/update/system.rs:181-213` (`apply_turn_aborted`)
- `crates/runie-core/src/update/session.rs:199-271` (`apply_queue_delivery_sync`)

## Acceptance criteria

- Core update handlers no longer mutate `AgentState` fields directly.
- Turn lifecycle state transitions are sent to `TurnActor` and applied via events.
- Queue delivery is owned entirely by `TurnActor`; the sync fallback is removed.

## Tests

### Layer 1 — State/Logic
- `AppState` projection rebuilds from a sequence of `TurnState` events deterministically.
- `TurnState` state-machine transitions pass unit tests.

### Layer 2 — Event Handling
- Feeding `Event::TurnAborted`, `Event::QueueAborted`, `Event::SteeringDelivered`, `Event::FollowUpDelivered` into `dispatch_event` updates only the projection fields derived from `TurnState`.
- No direct `agent_state_mut()` mutation occurs in update handlers (assert via code search).

### Layer 4 — Provider Replay / Mock-Tool E2E
- A multi-tool replay turn still emits the expected `TurnComplete` and `QueuesCleared` events with no direct `AgentState` mutation.
