# p50 — Pi telemetry actor boundary

Status: actor-owned lifecycle, structured attributes/events, callback-scoped
settlement, structured errors, provider projection, and YAML runtime replay
implemented; full Pi telemetry conformance remains (2026-08-07)

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

Runie carries serializable provider metadata through `HttpRequest` and keeps
telemetry as a separate actor-owned capability. The boundary is:

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
   rejection tests. Events retain structured attributes.
2. Define a capability-oriented `TelemetryContext` adapter for provider
   actors; keep it separate from `HttpRequest` serialization. **Partial:**
   `SimpleStreamOptions.telemetry` carries a cloneable `TelemetryActor`, and
   `ProviderActor` opens `pi.provider.stream`, records one `assistant.event`
   per streamed event, and acknowledges terminal status/end. Startup failures
   close the span with `Error`; the capability is not serialized.
3. Emit core lifecycle events for span start, event, exception, and end. **In
   progress:** event attributes and mutable span attributes now use
   acknowledged actor commands, and `TelemetryActor::with_span` settles
   callback success/error through the actor. Exception/error detail and
   callback nesting remain open. `SpanError` preserves Pi error name/message
   details through actor status commands. All mutable span state remains inside
   the telemetry actor. Settlement defaults unset spans to `ok` and assigns a
   deterministic `end_sequence`, matching Pi's detached in-memory recordings.
   Child creation under a settled parent is rejected inside the actor, matching
   Pi's no-op settled-span context behavior.
   `TelemetrySpan::with_child` now provides the corresponding nested
   callback-scoped API with automatic settlement.
4. Add a YAML runtime fixture with declared span commands and ordered snapshot
   assertions. **Done:** `TelemetryAction`, `TelemetryScenario`, and the
   runtime-discovered `tests/telemetry_replay.rs` execute YAML actions through
   the actor mailbox; fixture edits do not require recompiling a scenario test.
5. Add in-memory conformance vectors for success, async failure, nested spans,
   late child rejection, and exporter absence, without sleeps.

## Acceptance evidence

- Pi telemetry conformance vectors pass against the Runie in-memory adapter.
- YAML replay observes ordered span lifecycle state through an actor snapshot.
- Provider requests preserve the telemetry capability without serializing it.
- `just ci` and the existing Pi/TUI parity gates remain green.

The provider projection is covered by
`provider_stream_projects_telemetry_through_owned_capability`; full Pi
callback nesting, exceptions, exporter behavior, and YAML-declared span
conformance vectors remain open.

Typed DSL increment (2026-08-07): `telemetry_replay!` provides a compact
Rust-side declaration for small adapter tests. It expands only to
`TelemetryAction` values; runtime scenarios continue to use the YAML replay
fixture so edits do not require recompilation.

Until these conditions exist, p37 and p19 must continue to classify telemetry
parity as open; no placeholder field should be presented as implementation.
