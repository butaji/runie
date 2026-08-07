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
- `visual-operation-admission.yaml` exercises malformed and duplicate lane
  events through the real session actor; invalid records do not enter the
  projection.
- `just ci` runs the event-sequence, YAML replay, actor-boundary, and visual
  parity suites without sleeps.
- The remaining direct field assignments found by the audit are reducer-local
  mutations, immutable snapshot rehydration, or explicitly test-only
  compatibility adapters.

## Required follow-up slices

1. Retire the synchronous `EventRenderer` compatibility reducer after all
   replay callers use actor snapshots; retain only pure event-to-message
   helpers where they are still useful.
2. Give rejected session-lane admissions an explicit actor-owned outcome/event
   if Pi's adapter exposes that fact, rather than relying only on silent
   projection omission.
3. Replace generic `(record_type, data)` operation transport with a typed
   internal union while preserving Pi JSONL compatibility at the wire edge.
4. Implement provider-specific transport adapters only where the Pi source
   contract supplies their envelope, lifecycle, cancellation, and error
   events; generic HTTP must not emulate unsupported transports.
5. Add YAML assertions for every new rejection, admission, and snapshot
   transition before changing production behavior.

## Non-negotiable checks

- No cross-actor direct mutation.
- Every `tokio::spawn` has an owned lifetime.
- Rendering is pure and non-blocking.
- State-changing tests use event/message sequences and state assertions.
- `just ci` passes before each owned commit.

