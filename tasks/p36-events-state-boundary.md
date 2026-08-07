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

## Async ownership audit (2026-08-06)

All production task creation is owned:

- `LoopActor` stores the active loop `JoinHandle` until `wait_for_idle`/prompt
  completion consumes it.
- Core actor workers and event subscriber bridges retain `TaskOwner` handles.
- `App::spawn_renderer` returns the renderer `JoinHandle` to its caller.
- YAML recorder and pending-run tasks are joined before their scenario returns.
- The source lint rejects unannotated `tokio::spawn` sites; intentional test or
  orchestration sites carry an adjacent `OWNER` declaration.

This audit preserves the invariant that dropping an actor or scenario cannot
leave an orphaned task mutating shared state.

## Pi model contract increment (2026-08-06)

Pi's `Model` exposes optional `samplingParams?: Record<string, unknown>`.
Runie now preserves this as `Model::sampling_params` at the serde boundary,
with a round-trip test proving the camelCase wire key and arbitrary JSON value
shape. Provider adapters can now receive the same model defaults as Pi.

The loop also merges those model defaults with per-request
`SimpleStreamOptions::sampling_params`, using request values as the winning
layer. The merge is pure and covered by a focused core test, keeping provider
configuration state inside the loop's owned option snapshot.

Pi's `timeoutMs` is now carried as `SimpleStreamOptions::timeout_ms` and
enforced by the async `HttpActor` boundary. Timeout cancellation is covered
with a pending-future test; no blocking sleep is used.

`maxRetries` is likewise carried as `SimpleStreamOptions::max_retries` and
implemented as bounded, actor-local retry attempts around the async transport.
The deterministic flaky transport fixture proves two failures followed by a
successful third attempt, without sleeps or detached tasks.

The TUI replay schema now exposes `provider_options` for
`timeout_ms`/`max_retries`/`sampling_params`; the `visual-hey.yaml` fixture
uses the sampling field, proving these provider settings can be edited and
replayed from YAML without recompiling the runner. Its `assertions` block now
also verifies the effective options received by the provider stream, rather
than merely validating YAML deserialization.
