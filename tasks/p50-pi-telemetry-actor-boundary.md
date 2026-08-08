# p50 — Pi telemetry actor boundary

Status: actor-owned lifecycle, structured attributes/events, callback-scoped
settlement, structured errors, provider projection, abort settlement, optional
exporter, and YAML runtime replay implemented; full Pi telemetry conformance
remains (2026-08-08)

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
   `ProviderActor` opens the source-defined `pi.ai.request` span with required
   operation/provider/model/API/streaming attributes, records one `assistant.event`
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
   Span allocation starts at `1`, matching Pi's in-memory recorder IDs, and
   the YAML fixtures assert the resulting parent links.
   Active spans default to `ok` at creation, matching Pi's in-memory span
   recorder rather than exposing an intermediate unset status.
   Callback failures now preserve a structured `Error` name/message in the
   actor snapshot instead of discarding the callback error payload.
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
callback nesting, exceptions, exporter backend conformance, and YAML-declared
span conformance vectors remain open. The actor now also enforces the source
schema's absence of events on `pi.ai.request`: event mutations are ignored for
that span while extension spans retain the generic event API, with a focused
regression covering both cases.

Telemetry timing correction (2026-08-08): the authoritative Pi telemetry
README states that normalized in-memory spans record no timestamps. Runie’s
deterministic `end_sequence` is therefore the correct replay identity for
settlement ordering; adding wall-clock start/end fields would be a false
parity feature. The remaining telemetry work is backend/exporter and complete
conformance-vector coverage.

Concurrent-child conformance increment (2026-08-08): Runie now has an owned
task/channel regression matching Pi’s concurrent child callback case. It
asserts both child parent IDs and the settlement order
`second-child < first-child < parent` without sleeping or reading wall-clock
time.

Callback-error conformance increment (2026-08-08): callback-scoped spans now
have a regression for both synchronous and asynchronous failures. It asserts
that the exact returned error values are preserved while each span settles
once with the corresponding Pi error status and message.

Synchronous callback increment (2026-08-08): `TelemetryActor::with_span_sync`
and `TelemetrySpan::with_child_sync` now cover Pi's synchronous callback return
shape in addition to the existing async APIs. The nested-span regression pins
parent linkage and actor-owned settlement for that path.

Exporter increment (2026-08-08): `TelemetryActor::new_with_exporter` accepts an
optional actor-owned async exporter. A settled span exports the immutable
snapshot after the reducer marks it ended; exporter failures do not mutate or
rewind telemetry state. The no-exporter constructor remains the default, and a
regression receives the settled snapshot through an async channel without
sleep-based synchronization.

Exporter failure conformance (2026-08-08): a failing exporter regression now
proves that the settled span remains present, ended, and status-preserving in
the actor snapshot when the backend rejects export.

Telemetry source reconciliation (2026-08-07): the checked-out Pi telemetry
interface exposes `addEvent(name, attributes)` and
`setStatus({ status, error })`; it does not expose a separate
`recordException` operation. Runie's `TelemetryAction::Event` and
`TelemetryAction::Status { error }` therefore map the actual contract rather
than inventing an exception command. Runtime YAML already covers nested span
callbacks, structured terminal errors, settled-span late mutations, and
parent rejection. Remaining telemetry parity is limited to Pi's typed schema
vocabulary and exporter integration, which require a concrete capability
adapter before generic actor code can implement them.

Typed DSL increment (2026-08-07): `telemetry_replay!` provides a compact
Rust-side declaration for small adapter tests. It expands only to
`TelemetryAction` values; runtime scenarios continue to use the YAML replay
fixture so edits do not require recompilation.

Until these conditions exist, p37 and p19 must continue to classify telemetry
parity as open; no placeholder field should be presented as implementation.

Provider abort settlement increment (2026-08-09): `ProviderActor` now retains
the active request span through its owning worker. Explicit cancellation and
supersession settle that span with a structured `AbortError` and end it through
the telemetry actor before acknowledging the control command. This closes the
previous dropped-span lifecycle gap without making the renderer or transport
actor mutate telemetry state directly; a pending-provider regression proves the
ended error snapshot without sleeps.

Typed schema increment (2026-08-09): `validate_pi_ai_request_attributes`
enforces Pi's required provider-span attributes, operation closed set, and
primitive types. The provider actor validates its `pi.ai.request` attributes
before creating the span; extension/general spans remain accepted by the
generic telemetry actor. This closes schema validation for the provider span,
while the complete Pi harness schema and backend exporter conformance remain
open.

Settled-child callback increment (2026-08-09): Pi's in-memory context executes
the callback passed to a child span even when the parent has already settled,
using a detached no-op span. `TelemetrySpan::with_child` now preserves that
passive callback behavior while rejecting any late recorded span or mutation;
the regression pins callback execution and the unchanged span count.

Attribute passivity increment (2026-08-09): telemetry start, event, and
attribute-update payloads now enforce Pi's primitive/homogeneous-array
attribute contract. Invalid starts produce no recorded span; invalid mutable
updates are ignored atomically, matching Pi's passive in-memory recorder.
`invalid-attributes.yaml` exercises the same behavior through runtime replay.

Root no-op callback increment (2026-08-09): invalid root span attributes now
still execute `TelemetryActor::with_span` callbacks with an inert span, while
leaving the actor snapshot unchanged. This matches Pi's passive
`startSpan`/`NOOP_TELEMETRY_CONTEXT` behavior and is covered by a focused
regression.

Explicit-status settlement increment (2026-08-09): spans now retain whether a
status was explicitly set. Callback failure only supplies automatic error
status when no explicit status exists, matching Pi's preservation of an
explicit `ok` or structured `error` status; YAML status fixtures assert the
new snapshot fact.

Typed end-schema increment (2026-08-09): the provider telemetry boundary now
validates Pi's `pi.ai.request` end attributes, including the closed normalized
stop-reason vocabulary and numeric usage/transport fields, before projecting
terminal event metadata. Unknown or invalid end fields are rejected by the
typed validator; generic extension spans remain schema-agnostic.
`pi-ai-request-end.yaml` adds runtime-editable replay coverage for the
provider-shaped start and terminal attribute projection.

Status vocabulary reconciliation (2026-08-08): Pi's public telemetry callback
status is the closed `ok`/`error` vocabulary. Runie's internal `Unset` value is
retained because spans begin unsettled and settlement defaults an unmodified
span to `ok`; it is not exposed as a Pi callback status. Removing it would
collapse an actor lifecycle state rather than improve source parity.

Stream chunk-count increment (2026-08-09): the owned provider pump now counts
non-terminal assistant update events and publishes Pi's
`pi.ai.stream.chunk_count` at terminal span settlement for both successful and
provider-error streams. The provider telemetry regression asserts the
actor-owned count; start, done, and error envelope events are excluded from
the update count.

First-chunk timing increment (2026-08-09): the same owned pump records
`pi.ai.stream.time_to_first_chunk_ms` when the first assistant update arrives
and publishes it with terminal telemetry attributes. Streams with no update
chunk omit the optional field; the provider regression verifies the numeric
attribute without using sleeps or a timing threshold.

Diagnostic classification increment (2026-08-09): provider startup/deferred
failures now set `pi.ai.error.type=provider`, while actor-owned abort
settlement sets `pi.ai.error.type=abort`, preserving the source schema's
low-cardinality diagnostic distinction alongside structured span errors.

Response identity increment (2026-08-09): terminal assistant payloads now
project available `response_model` and `response_id` values into Pi's
`pi.ai.response.model` and `pi.ai.response.id` end attributes. Missing or empty
identity remains omitted rather than inferred from request metadata.

HTTP status projection (2026-08-08): provider errors carrying Pi's HTTP status
now preserve it as `pi.ai.http.status_code` on the terminal request span. The
actor regression covers a `429` provider response without weakening generic
transport error handling.
