# p40 — Awaited lifecycle listener settlement

Status: complete (2026-08-07)

## Source evidence

Pi `packages/agent/src/agent.ts` documents that subscribed lifecycle listener
promises are awaited in registration order and that `waitForIdle()` resolves
only after `agent_end` listeners settle. Runie's `EventBus` currently uses a
fire-and-forget `broadcast::Sender`; `LoopActor`'s subscriber bridge dispatches
events in its owned worker, but the run itself has no acknowledgement barrier.

Therefore current event capture proves ordering of published events, not Pi's
listener-settlement contract.

## First implementation slice (2026-08-06)

`RunLoopDeps` now carries the actor-owned `SubscriberRegistry`. Every emitted
Pi event dispatches through the registry inline after state publication, so
registration order and async completion are part of loop progress. The
production `LoopActor` no longer starts the old broadcast-to-registry bridge;
the broadcast bus remains observational and cannot duplicate lifecycle
callbacks.

The common `Subscriber` contract receives the converted Pi event and is the
awaited lifecycle listener surface; `PiSubscriber` remains an additional
closed-wire adapter and is dispatched separately. `loop_entry.rs` proves an
`AgentEnd` listener can hold prompt settlement until a watch-channel release.
The common listener hook also receives the actor-owned abort projection, with
direct registry coverage proving an already-aborted signal is observable.
The YAML runner now records the same awaited listener path separately from the
broadcast trace; `hello-streaming.yaml` asserts its complete lifecycle order.

## Completion (2026-08-06)

The required implementation is complete: the ordered actor-owned registry is
awaited inline by the loop, `agent_end` settlement includes listener completion,
abort is projected into listeners, and YAML asserts the listener sequence.
The broadcast bus remains observational and is not used as the lifecycle
acknowledgement boundary.

## Required implementation (historical design)

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
