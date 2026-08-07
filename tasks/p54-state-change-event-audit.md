# p54 — State-change event boundary audit

Status: in progress — live actor paths audited; compatibility retirement and
provider-specific event admission remain open (2026-08-08)

## Objective

Every durable or interactive state change in Runie must follow:

```text
event/message → owning actor reducer → immutable snapshot → pure renderer
```

The event must be the transfer mechanism; a renderer, sibling actor, or
caller must not mutate another owner directly. YAML replay is the preferred
acceptance surface, and Rust macros may reduce declaration boilerplate without
hiding the reducer or delivery boundary.

## Current evidence

- `AgentStateActor`, `SessionActor`, `SessionStorageActor`, `LoopActor`,
  `StatusActor`, `ScrollbackActor`, `PromptActor`, `UiActor`, and telemetry
  actors publish acknowledged snapshots from owned mailbox reducers.
- `scripts/validate-feed-actor-boundary.py` rejects terminal/widget imports in
  the feed actor and guards the renderer-independent feed ownership boundary.
- The same validator rejects synchronous `EventRenderer::apply_event` and
  mutex-owned legacy projection symbols, preventing the retired delivery path
  from returning through a future compatibility change.
- `visual-operation-admission.yaml` exercises malformed and duplicate lane
  events through the real session actor; invalid records do not enter the
  projection.
- `just ci` runs the event-sequence, YAML replay, actor-boundary, and visual
  parity suites without sleeps.
- The remaining direct field assignments found by the audit are reducer-local
  mutations, immutable snapshot rehydration, or explicitly test-only
  compatibility adapters.

## Required follow-up slices

1. Give rejected session-lane admissions an explicit actor-owned outcome/event
   if Pi's adapter exposes that fact, rather than relying only on silent
   projection omission.
2. Replace generic `(record_type, data)` operation transport with a typed
   internal union while preserving Pi JSONL compatibility at the wire edge.
3. Implement provider-specific transport adapters only where the Pi source
   contract supplies their envelope, lifecycle, cancellation, and error
   events; generic HTTP must not emulate unsupported transports.
4. Add YAML assertions for every new rejection, admission, and snapshot
   transition before changing production behavior.

## Typed operation migration slice (2026-08-08)

Queue lifecycle producers now use the closed internal `QueueRecordKind` enum;
only its `wire_name()` conversion emits Pi-compatible `queue_enqueued` and
`queue_cancelled` strings. The session/event compatibility shape remains
unchanged, so this slice removes producer-side string drift without creating a
second journal representation. The remaining migration is to carry the same
typed fact through `AgentEvent` and the session reducer before the JSONL edge.

The loop driver now applies the same producer-side boundary to
`operation_started` and `operation_finished` through `OperationRecordKind`.
Both closed enums have focused wire-name tests; the macro-based session-lane
constructor remains available for fixed families, while dynamic producer
values are intentionally converted only at this single event edge.

## Non-negotiable checks

- No cross-actor direct mutation.
- Every `tokio::spawn` has an owned lifetime.
- Rendering is pure and non-blocking.
- State-changing tests use event/message sequences and state assertions.
- `just ci` passes before each owned commit.
