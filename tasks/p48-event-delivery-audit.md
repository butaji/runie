# p48 — Event-delivery audit and closure criteria

Status: audited; compatibility retirement remains (2026-08-07)

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

## Direct-write audit (2026-08-07)

The production paths were re-scanned after the measured-layout event work. The
remaining assignments are reducer-local writes or compatibility-only adapters:

- `runie-core` writes mutable fields only inside state/session/queue/tool actor
  reducers, after an actor command or `AgentEvent`.
- `runie-tui-model` writes `FeedState`, `StatusState`, and `UiState` only in
  pure message reducers; renderers consume immutable snapshots.
- `App`, `EventRenderer::run`, and live widgets do not write another actor's
  state. Layout facts use `ScrollbackMsg::LayoutMeasured` and the actor
  mailbox.
- `EventRenderer` transient fields (tool rows, stream buffers, activity
  counters, and lifecycle flags) are reachable only from synchronous
  compatibility/replay adapters. `with_live_actors` disables those reducers;
  live facts come from actor snapshots.
- Direct widget assignments are reducer internals or snapshot hydration, not
  application-level state changes.

This remains open by design: removing the compatibility mirror requires
migrating each replay caller to an actor-backed snapshot without changing its
deterministic YAML contract. Acceptance remains the full YAML suite plus a
source audit proving that `with_live_actors` cannot access `Projection::Legacy`.

## Reactive timing audit (2026-08-07)

The interactive binary previously polled `LoopActor::state_snapshot()` every
50 ms and republished `ModelChanged`, even when no model transition occurred.
That bypassed the event source and created a synthetic event stream during the
render tick. The polling path is removed; model captions now change only from
the state actor's `ModelChanged` event (the deterministic startup caption is
still an explicit prompt command). Animation ticks only request actor-owned
animation advances.

The terminal input loop is a separate remaining gap: the binary still calls
`crossterm::event::poll(Duration::ZERO)` from the render cadence and reads one
key synchronously. This is observable polling, not an actor event stream. The
next migration must give an owned input worker a
`crossterm::event::EventStream` and send `InputEvent` messages through a
mailbox; the main loop should select between its render tick and that mailbox.
Until that migration is complete, the architecture objective is not fully
closed, even though YAML/reducer tests do not depend on a physical terminal.
