# p48 — Event-delivery audit and closure criteria

Status: audited; EventRenderer compatibility mirror retired; widget snapshot
adapter remains intentionally (2026-08-09)

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
- `EventRenderer` no longer contains reducer buffers or a mutable projection
  mirror. Live delivery and YAML replay both consume actor-backed snapshots;
  the remaining `Scrollback::from_model_snapshot` adapter is a read-only
  rehydration surface for legacy widget APIs, not an event/state owner.

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

## Historical closure slice

The compatibility `EventRenderer` state mirror tracked by
`p47-renderer-transient-state-migration.md` has been retired. Replay uses
`apply_actor_event`, which sends the same acknowledged actor messages as live
delivery and reads the actor snapshot once per event. The remaining widget
adapter is intentionally retained until callers no longer require its public
shape; deleting it would be an API removal, not an event-boundary migration.

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

The source audit remains useful as a regression guard: it proves that
`with_live_actors` cannot access a legacy renderer projection. The remaining
compatibility-only widget methods are covered by snapshot rehydration tests
and are not reachable from the production renderer path.

## Deferred snapshot oracle (2026-08-07)

The deferred-response replay now asserts the final assistant message's
actor-owned `stop_reason: deferred` and complete `DeferredHandle` from YAML.
This closes the previous evidence gap between deferred event delivery and the
state snapshot while keeping provider fetch/cancel capability errors explicit.

## Declarative replay contract (2026-08-07)

The YAML runner already provides the event-delivery assertions required by the
architecture: `exact_events` checks the complete ordered Runie event stream,
`pi_events` checks the closed Pi wire contract, and `listener_events` checks
the awaited subscriber path. `visual-hey.yaml` exercises all three against
the same scenario. This is the canonical event-sequence → state oracle; new
fixtures should extend these fields instead of adding test-specific code.

## Reactive timing audit (2026-08-07)

The interactive binary previously polled `LoopActor::state_snapshot()` every
50 ms and republished `ModelChanged`, even when no model transition occurred.
That bypassed the event source and created a synthetic event stream during the
render tick. The polling path is removed; model captions now change only from
the state actor's `ModelChanged` event (the deterministic startup caption is
still an explicit prompt command). Animation ticks only request actor-owned
animation advances.

The first input migration is now implemented: an owned worker reads
`crossterm::event::EventStream` and sends `KeyEvent` values through a bounded
mailbox. The render loop no longer calls `event::poll` or `event::read`, and
all prompt/UI state changes still cross actor mailboxes. The main loop now
selects directly on the input mailbox rather than `try_recv` from the render
tick. Key routing remains actor-mailbox based, while the render tick is
reserved for drawing and actor-owned animation progress.

## Production write audit (2026-08-07, continuation)

The current source scan found no new production cross-actor mutation. The
remaining `EventRenderer` `.lock().apply/set/append` calls are reachable only
from its synchronous compatibility adapter; the live constructor uses actor
snapshots and acknowledged actor messages. Core lock writes are confined to
the loop's owned run-handle slot, subscriber registry, replay/test captures,
or transport adapters. The feed boundary validator remains clean.

This audit does not promote the compatibility adapter to “done”: its writes
are isolated but still represent the final migration surface. Removing them
requires moving replay callers to actor-backed snapshots, not merely
suppressing the source check.

## Scrollback bus ownership increment (2026-08-07)

`ScrollbackActor` previously had a second mutable lifecycle projection inside
its bus bridge: tool IDs, headers, arguments, active counts, and activity
counters were reduced there before messages were sent to the feed actor. The
bridge now forwards `AgentEvent` values statelessly through the actor mailbox;
the owning actor reduces those events and owns all projection maps and
counters. Direct `ScrollbackMsg` batches and bus events therefore share one
reducer boundary, with acknowledged delivery and no cross-worker state mirror.

Lag visibility increment (2026-08-07): the status and scrollback bus bridges
previously discarded `broadcast::RecvError::Lagged` and continued as if the
event stream were complete. They now transfer an actor-owned diagnostic on
that boundary: status becomes an explicit error and the feed appends a system
row containing the skipped count. This keeps loss observable and prevents a
silent false-parity snapshot; the renderer's existing lag diagnostic remains
consistent with both actors. Recovery from a lagged broadcast still requires
a source snapshot/replay contract and remains an open reliability refinement.

- Closure evidence (2026-08-07): `StatusActor::new_with_bus` is now
  regression-tested by `bus_owned_actor_surfaces_lag_as_error_status` in
  `crates/runie-tui/src/status_actor.rs`, which keeps a `subscribe()`d
  receiver attached while publishing `BUS_CAPACITY + 1` events on the bus
  and asserts the first post-publish snapshot reports `Status::Error`
  containing `event stream lagged`. The mirror projection is covered by
  `bus_owned_actor_appends_system_row_on_lag` in
  `crates/runie-tui/src/scrollback_actor.rs`, which uses the same
  keepalive-plus-overflow pattern and asserts the first post-publish
  `FeedSnapshot` contains a `LineKind::System` row whose text contains
  `event stream lagged`. Both tests assert through the actor's
  `subscribe()`d snapshot, so the lag diagnostic is the property of the
  bus bridge, not the test setup.
