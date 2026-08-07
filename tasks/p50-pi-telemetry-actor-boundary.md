# p50 — Pi telemetry actor boundary

Status: actor-owned in-memory lifecycle implemented; provider integration and
YAML runtime adapter remain (2026-08-07)

## Source-backed Pi contract

Pi exposes `telemetryContext?: TelemetryContext` in
`/Users/admin/Code/agents/pi/packages/ai/src/types.ts`. The telemetry package
defines:

- `startSpan({ name, attributes }, callback)` as an async callback-scoped API;
- nested child spans created from the callback's `TelemetrySpan`;
- span status, attributes, events, and exception recording;
- context propagation through explicit parent spans; and
- an in-memory implementation used by conformance tests.

This is capability state, not request JSON. Converting it to `metadata` would
lose nesting, lifetime, callback error handling, and parent propagation.

## Runie boundary

Runie currently carries serializable provider metadata through `HttpRequest`,
but has no telemetry capability or actor-owned span lifecycle. The missing
boundary must be:

```text
LoopActor event
  → owned TelemetryActor span-start command
  → provider/stream actor callback scope
  → span event/status commands
  → actor-owned telemetry snapshot and optional exporter
```

The renderer must never own spans or infer telemetry from status text.

## Implementation plan

1. Add a `TelemetryActor` with an owned mailbox, nested span IDs, parent IDs,
   status, attributes, events, and deterministic completion ordering. **Done:**
   `crates/runie-core/src/telemetry.rs` provides acknowledged lifecycle
   commands, immutable watch snapshots, nested spans, and late-mutation
   rejection tests.
2. Define a capability-oriented `TelemetryContext` adapter for provider
   actors; keep it separate from `HttpRequest` serialization.
3. Emit core lifecycle events for span start, event, exception, and end. All
   mutable span state remains inside the telemetry actor.
4. Add a YAML runtime fixture with declared span commands and ordered snapshot
   assertions. Fixture edits must not require recompiling a scenario test.
5. Add in-memory conformance vectors for success, async failure, nested spans,
   late child rejection, and exporter absence, without sleeps.

## Acceptance evidence

- Pi telemetry conformance vectors pass against the Runie in-memory adapter.
- YAML replay observes ordered span lifecycle state through an actor snapshot.
- Provider requests preserve the telemetry capability without serializing it.
- `just ci` and the existing Pi/TUI parity gates remain green.

Until these conditions exist, p37 and p19 must continue to classify telemetry
parity as open; no placeholder field should be presented as implementation.
