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
3. Keep capture inputs declarative: the matrix now accepts a scenario prompt
   instead of hardcoding `Hey`; next, bind prompt and settled markers to the
   YAML scenario without recompilation.

The capture helper remains an external instrument, not production state. Its
bounded polling is intentionally limited to detecting terminal readiness and
settled output; it does not mutate Runie's state.
