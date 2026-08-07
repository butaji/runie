# p36 — Events are the state-transfer boundary

Status: in progress

Runie keeps mutable state inside its owning actor. Commands are actor-local
requests; durable state changes are transferred through `AgentEvent` (core)
or the actor's explicit message/event reducer (TUI). Renderers consume
snapshots only and never mutate another actor's state.

## Audit

- `AgentStateActor` reduces core events and publishes snapshots.
- `UiActor`, `PromptActor`, `StatusActor`, and `ScrollbackActor` own their
  models and publish immutable `watch` snapshots.
- `ScrollbackActor::new_with_bus` projects core events into feed events before
  reducing them; the renderer only reads the resulting `FeedSnapshot`.
- YAML replay already drives the same event path used by functional tests.
- Direct widget mutation is confined to actor workers (`PromptWidget` and the
  pure `FeedState` reducer); application code sends actor messages.

## Remaining work

1. Make every externally observable TUI transition have a named event/message
   variant, including scroll, selection, palette, and prompt actions.
2. Add event-trace assertions to replay fixtures so ordering is tested before
   snapshots are compared.
3. Keep capture inputs declarative: the matrix accepts a scenario prompt, and
   `capture-scenario.sh` reads prompt/quit settings from the YAML fixture at
   runtime without recompilation.

The matrix retains the original four-argument environment-assignment form;
the compatibility branch is covered by shell syntax/argument checks so older
capture recipes do not silently lose their color or parity-clock settings.

## Exhaustiveness hardening

`status_messages_for_event` now names every intentionally ignored outer
`AgentEvent` variant and every intentionally ignored assistant sub-event.
This removes the wildcard fallback at the status boundary: adding a Pi event
or assistant sub-event now fails to compile until its status projection is
classified. The same exhaustive-table treatment should be applied to the
remaining feed and UI projection tables as their compatibility paths are
retired.

The UI reset mapper and actor-owned feed bus mapper now use the same explicit
classification. New core events must therefore be assigned to a projection,
or deliberately listed as a no-op, before the workspace compiles.

The compatibility renderer's feed adapter and the actor's background/workflow
adapter are explicit as well. This keeps the legacy replay path and the live
actor path aligned while the renderer is being retired as a state owner.

The capture helper remains an external instrument, not production state. Its
bounded polling is intentionally limited to detecting terminal readiness and
settled output; it does not mutate Runie's state.
