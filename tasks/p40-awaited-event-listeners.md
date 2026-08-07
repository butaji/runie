# p40 — Awaited lifecycle listener settlement

Status: planned

## Source evidence

Pi `packages/agent/src/agent.ts` documents that subscribed lifecycle listener
promises are awaited in registration order and that `waitForIdle()` resolves
only after `agent_end` listeners settle. Runie's `EventBus` currently uses a
fire-and-forget `broadcast::Sender`; `LoopActor`'s subscriber bridge dispatches
events in its owned worker, but the run itself has no acknowledgement barrier.

Therefore current event capture proves ordering of published events, not Pi's
listener-settlement contract.

## Required implementation

Add a separate awaited lifecycle-delivery path for Pi-compatible events:

- preserve the existing broadcast stream for observational/UI subscribers;
- add an actor-owned ordered listener registry with per-event acknowledgements;
- make the loop await listener delivery, especially `AgentEnd`, before
  releasing the run permit and resolving `wait_for_idle`;
- propagate the run abort signal to listeners and retain owned task handles;
- keep Runie-only presentation events out of the Pi listener contract.

## YAML verification

Extend the scenario schema with deterministic listener operations, not timing
delays. A fixture should register ordered listeners, emit a terminal event,
and assert the recorded listener order plus `wait_for_idle` settlement. The
listener fixture must use channels/barriers rather than `sleep()` and must
remain runtime-editable YAML.
