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

1. **Unit tests** — `AppState` projection rebuilds deterministically from `TurnState` events; `TurnState` state-machine transitions are covered.
2. **E2E tests** — A mock-provider replay turn emits the expected `TurnComplete` and `QueuesCleared` events with no direct `AgentState` mutation.
3. **Live run tests** — A multi-tool turn in tmux updates turn/queue state correctly without direct state write anti-patterns.

## Tests

### Unit tests
- `AppState` projection rebuilds from a sequence of `TurnState` events deterministically.
- `TurnState` state-machine transitions pass unit tests.

### E2E tests
- Feeding `Event::TurnAborted`, `Event::QueueAborted`, `Event::SteeringDelivered`, `Event::FollowUpDelivered` into `dispatch_event` updates only the projection fields derived from `TurnState`.
- No direct `agent_state_mut()` mutation occurs in update handlers (assert via code search).

### Live run tests
- Run a streaming multi-tool turn in tmux and verify `turn_active`, `inflight`, and queue state update correctly.
