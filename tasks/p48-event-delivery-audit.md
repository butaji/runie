# p48 — Event-delivery audit and closure criteria

Status: audited; implementation follow-up remains (2026-08-07)

## Invariant

Every externally observable state change in Runie must enter through an event
or an owner-local actor message, be reduced by exactly one owning actor, and be
published as an immutable snapshot for rendering. Renderers may derive layout
and styling from snapshots, but may not write another actor's state.

## Current source-backed result

- `runie-core` lifecycle, state, queue, provider, tool, and session actors
  receive commands and publish `AgentEvent` or actor snapshots.
- `runie-tui` `UiActor`, `PromptActor`, `StatusActor`, and `ScrollbackActor`
  reduce their own message streams and expose `watch` snapshots.
- `ScrollbackActor::new_with_bus` is the live feed projection boundary; live
  tool and assistant presentation state is read from its `FeedSnapshot`.
- YAML replay drives the same event path as functional tests and can assert
  both event order and resulting actor state without recompilation.
- `EventRenderer` still contains compatibility reducer buffers for replay
  callers. They are not live state owners and must be removed only after all
  compatibility callers consume actor snapshots.

## Explicitly allowed mutation

Mutation inside an actor reducer is allowed: it is the implementation of
event-to-state folding. Widget-local mutation is allowed only while folding an
owner-local message inside its actor worker or a pure compatibility reducer.
Direct mutation from application orchestration or rendering is not allowed.

## Required checks for each new feature

1. Name the event/message and its owning actor before adding fields.
2. Add a pure reducer/state assertion for the event.
3. Add a YAML replay sequence when the transition is serializable.
4. Assert ordering and resulting snapshot state; do not use sleeps.
5. Render only from the resulting snapshot and verify with the relevant visual
   fixture.
6. Run `just ci`, including the actor-boundary and source-inventory checks.

## Next closure slice

Retire the compatibility `EventRenderer` state mirror tracked by
`p47-renderer-transient-state-migration.md`. Before deleting each field, move
its replay projection to the owning actor, add an event-order assertion, and
prove that live and replay renders consume equivalent snapshots. Do not solve
this by introducing a second shared mutable model.

