# p37 — Pi provider transport boundary

Status: in progress (HTTP/replay boundary complete; provider capability seam and YAML route coverage complete; Codex WebSocket adapter lifecycle, production wiring, and source-aligned retry boundary implemented; provider-specific deferred polling remains open)

The Pi `ProviderRequestOptions` contract is broader than Runie's current
`HttpActor` abstraction. This task records the exact boundary so parity work
does not add fields that are merely deserialized and then ignored.

## Source-backed inventory

Pi's shared request options include abort signal, API key, telemetry context,
custom fetch, provider-scoped environment, payload/response hooks, nullable
headers, timeout, retry count, and maximum retry delay. Stream options add
temperature, arbitrary sampling parameters, max tokens, transport selection,
and cache retention. The authoritative definitions are:

- `/Users/admin/Code/agents/pi/packages/ai/src/types.ts` —
  `ProviderRequestOptions` and `StreamOptions`.
- `/Users/admin/Code/agents/pi/packages/ai/src/api/simple-options.ts` —
  provider request construction and option merging.
- `/Users/admin/Code/agents/pi/packages/ai/src/api/*.ts` — provider-specific
  support and intentional ignore behavior.

## Runie coverage

Implemented end-to-end through the owned provider request snapshot:

- abort signal
- API key
- session ID
- thinking budgets
- model `samplingParams` plus per-request `sampling_params` merge
- `timeoutMs`
- `maxRetries`
- payload and response hooks
- request headers carried to the transport boundary
- provider environment and metadata carried to the transport boundary
- preferred transport carried to the transport boundary
- model headers merged into request headers with request values taking
  precedence, matching Pi's provider construction
- model `maxTokens` propagated as the default request output limit, with an
  explicit request value taking precedence
- explicit `temperature` carried separately from arbitrary sampling parameters
- typed cache-retention preference carried to the owned HTTP request boundary

The YAML runner exposes these effective options at runtime; `visual-hey.yaml`
now declares and asserts `session_id`, thinking budgets, and sampling
parameters.

`visual-hey.yaml` now declares model defaults and request overrides, and its
provider assertion verifies the effective merged header map at runtime without
recompilation.

The same fixture declares model `maxTokens: 128` and request `max_tokens: 64`,
asserting the request-level value wins in the effective provider options.
It also asserts the distinct request temperature field rather than inferring it
from `sampling_params`.
The fixture now carries and asserts `cache_retention: long`; provider-specific
prompt-cache marker generation remains adapter-owned and is not fabricated by
the generic HTTP actor.

Not yet implemented behaviorally:

- custom fetch is represented by the injected `HttpActor`; no browser-style
  fetch callback is needed at this Rust boundary.
- telemetry context remains unsupported because Pi's value is a live span
  capability (`startSpan`), not serializable request data; it requires a
  concrete telemetry backend and lifecycle contract before promotion.
- transport selection is now typed and carried through `HttpRequest`; the
  provider-scoped `CodexWebSocketAdapter` consumes WebSocket requests with an
  injected connector. Generic HTTP remains transport-neutral and does not
  emulate provider-specific WebSocket behavior.
- nullable request headers: additive `HttpRequest` delivery now carries
  optional headers; the default adapter preserves existing body-only actors.
- `maxRetryDelayMs`: retry delay is capped by the option, with provider
  `retry-after` metadata taking precedence before the cap and injectable
  delay/jitter hooks keeping replay deterministic.
- provider-specific temperature/cache marker generation: the generic actor
  now carries both options, but concrete payload mapping still requires an
  adapter contract and must not be inferred from transport metadata alone.
- WebSocket open-handshake timeout is carried as separate request metadata and
  consumed by the Codex adapter's owned connector; the generic HTTP actor does
  not open WebSockets.

## Next implementation slice

The owned `HttpRequest` now carries body, headers, environment, metadata,
transport, and cancellation into `HttpActor`. Add deterministic clock and
backoff policy as injected dependencies, then promote each Pi option only when
one concrete adapter consumes it and a YAML event sequence asserts the
effective request. Keep unsupported provider-specific options explicitly
classified until that boundary exists.

The YAML fixture asserts `websocket_connect_timeout_ms: 2500`, preserving this
Pi option for a future WebSocket adapter without claiming behavioral support.

## YAML capability-route coverage (2026-08-08)

The replay harness now injects its deterministic `ScenarioStream` through both
the ordinary `StreamFn` capability and the provider-scoped `WebSocketAdapter`
capability. `provider-websocket.yaml` selects `transport: websocket` and
asserts the effective transport, handshake timeout, complete Pi event order,
and resulting transcript. This proves the event/state route without pretending
that the fixture implements Codex's real socket URL, envelope, decoder,
continuation cache, fallback policy, or cleanup semantics. Those remain
provider-adapter work and cannot be supplied by generic YAML replay.

## WebSocket source audit (2026-08-07)

Scanning every Pi AI source reference shows that WebSocket is not a generic
HTTP concern. Pi currently implements it only in
`packages/ai/src/api/openai-codex-responses.ts`, where the provider adapter:

- resolves a provider-specific WebSocket URL from `model.baseUrl`;
- builds provider-specific headers and sends a Responses request envelope;
- opens the socket with an injectable/runtime WebSocket constructor;
- applies the open-handshake timeout only until the socket opens;
- parses socket messages into the same assistant event stream as SSE;
- retries selected pre-stream failures, including connection-limit and missing
  continuation cases;
- records per-session debug statistics and activates an SSE fallback after a
  failed WebSocket attempt; and
- owns cached socket lifetime and cleanup for session/account pairs.

This means adding `tokio-tungstenite` to the generic `runie-core` HTTP actor
would be incorrect: it would have no provider URL resolver, wire envelope,
message decoder, continuation cache, fallback policy, or actor-owned cleanup
contract. Runie currently carries the transport facts through events and
`HttpRequest`; the next valid implementation is a provider-scoped
`OpenAICodexWebSocketActor` (or an injected WebSocket adapter) with those
responsibilities explicit. Until that adapter exists, selecting a WebSocket
transport must remain an observable typed request fact and must not silently
pretend to stream over HTTP.

The event boundary required for that adapter is:

`ProviderCommand::Request` → owned transport actor → `AgentEvent` stream →
provider/session/status/feed actors.

Socket lifecycle, fallback state, and debug counters belong to the transport
actor; no renderer, loop driver, or sibling actor may mutate them directly.

The current `ProviderActor` remains intentionally transport-neutral: it owns
one `StreamFn`, exposes only acknowledged `Start`/`Cancel` commands, and
publishes assistant events from an owned pump. It must not infer a WebSocket
wire protocol from `ProviderTransport` or silently reinterpret a WebSocket
request as SSE. A provider adapter can therefore be added behind the existing
`StreamFn` boundary without changing the Pi event contract or creating a
second state owner.

Codex wire seam increment (2026-08-09): runie-core now provides pure helpers
for source-backed /codex/responses URL resolution, https/http to wss/ws
conversion, the Responses WebSocket beta header, and the response.create
envelope tag. Unit tests pin URL edge cases and object-only envelope
validation. Socket acquisition, continuation caching, fallback, and cleanup
remain provider-adapter responsibilities.

Responses decoder seam increment (2026-08-09): `ReplayProvider::from_websocket_messages`
now feeds provider-scoped Codex WebSocket text messages through the same
source-aligned Responses event decoder used by SSE replay. A regression pins
created, text-delta, and completion ordering, and malformed/non-object frames
are rejected like Pi's WebSocket parser. This is decoder coverage only;
socket acquisition, continuation caching, pre-stream retry, SSE fallback, and
session/account cleanup remain concrete adapter responsibilities.

Transport audit continuation (2026-08-07): the source-to-boundary comparison
was rechecked against Pi's `processWebSocketStream`, continuation cache, and
session cleanup helpers. The remaining implementation data is concrete and
not present in Runie's generic model: a provider-scoped WebSocket constructor,
Codex Responses envelope/decoder, session/account cache key, fallback decision
events, and owned close/cleanup events. Until those facts are introduced as a
provider adapter contract, the correct event behavior is the existing explicit
`Invalid` result for WebSocket selection; routing it through HTTP would create
false parity and violate the actor-owned transport boundary.

Wire metadata preservation (2026-08-07): the generic `WireMessage::ToolResult`
projection now carries Pi `details`, `usage`, and `added_tool_names` instead
of dropping them during `default_convert_to_llm`. The reverse loop projection
restores the same fields, and the core round-trip test asserts all three. This
keeps provider-specific deferred-tool encoding possible without coupling the
generic converter to one provider protocol.

Simple-stream option parity (2026-08-07): `SimpleStreamOptions` now retains
Pi's optional `reasoning` override and deferred-response request mode,
including the `15m`/`1h`/`24h` windows. YAML provider options deserialize and
carry both values through the owned provider request snapshot; unsupported
provider behavior remains explicitly adapter-owned.

Deferred provider capability audit (2026-08-07, reconciled): Pi exposes
optional `fetchDeferred(model, handle, options)` and
`cancelDeferred(model, handle, options)` operations, with provider-specific
polling/stream decoding and cancellation. Runie now has the corresponding
`ProviderCommand::FetchDeferred|CancelDeferred` mailbox boundary, explicit
unsupported adapter errors, and joined owned stream pumps. Provider-specific
polling/decoding remains open; generic HTTP still must not emulate it.

Deferred operation re-audit (2026-08-07): Pi's `fetchDeferred` and
`cancelDeferred` remain optional provider capabilities, not generic transport
behavior. Runie's `ProviderActor` routes both commands through the owned
`StreamFn` adapter, joins fetch pumps in its `JoinSet`, and returns explicit
capability errors when the adapter does not implement them. There is no
source-backed generic YAML behavior to add without inventing a wire protocol.
The next valid increment is a provider-specific adapter fixture implementing
Pi's handle/poll/decoder contract; until then the unsupported capability
result is the correct actor boundary and is covered by the provider actor unit
test.

Deferred capability scope re-audit (2026-08-08): Pi exposes these operations
through its provider/models layer, while the agent loop consumes the resulting
assistant event stream. Runie already preserves the same boundary with owned
`ProviderActor::fetch_deferred`/`cancel_deferred` commands, explicit unsupported
adapter errors, joined pumps, and YAML coverage for deferred handles and stop
reasons. Adding a second `LoopActor` deferred state machine would create a
parallel owner; the next valid increment remains a provider-specific adapter
with its polling, decoding, cancellation, and lifecycle events.

Deterministic deferred replay increment (2026-08-08): `ReplayProvider` now
accepts a provider-scoped decoded deferred event stream and implements
`fetch_deferred` through the existing `StreamFn` capability. Handle identity
is validated before the owned pump is returned; ordinary replay traces remain
unchanged. This closes deterministic adapter coverage without claiming a
provider's HTTP polling protocol or expiry semantics.

Provider lifecycle increment (2026-08-07): `ProviderActor` now aborts any
previous owned pump before acknowledging a new `Start`. This matches the
one-in-flight Pi turn contract and prevents superseded streams from publishing
events concurrently. The actor test suite covers both explicit cancellation
and replacement by a new start; no detached task or timing sleep is used.

Provider lifecycle snapshot audit (2026-08-07): a generic `active` snapshot
would be false parity because Pi's observable lifecycle differs by adapter:
ordinary streams settle on assistant-event completion, deferred fetches have
provider polling/expiry semantics, and Codex WebSocket streams have fallback
and cached-session state. The generic Runie actor therefore continues to own
only command admission and `JoinSet` pump cleanup. A truthful provider
lifecycle snapshot must be introduced together with an adapter contract that
supplies terminal, deferred, fallback, and diagnostic events; adding a
generic boolean before those events exist would create a second, incorrect
state model.

## Typed request promotion (2026-08-06)

The owned `HttpRequest` now receives the effective `session_id`, API key,
explicit `temperature`, `max_tokens`, and merged `sampling_params` as typed
fields.
This closes a real loss-of-information boundary: concrete adapters no longer
need to recover Pi options from serialized payloads or opaque metadata. The
merge remains pure at request construction time (model defaults followed by
request overrides), and the transport test captures the resulting request
without timers or sleeps.

Provider-specific payload encoding is still intentionally adapter-owned; this
change transports the facts needed by an adapter and does not invent a generic
wire format. The API key remains an owned request field and is not copied into
the body or implicitly converted into a header by the generic actor.

YAML closure (2026-08-06): `visual-hey.yaml` now declares `api_key` and asserts
the effective option through the real loop/provider recorder. The assertion
never prints the secret, and the fixture remains runtime-discovered, so option
changes do not require recompilation.

## Verification contract

Each promoted option requires a pure option reducer/merge test, a transport
observation test, and a YAML replay assertion. Tests must use event sequences,
must not sleep, and must preserve actor ownership of mutable request state.

## Unsupported transport safety increment (2026-08-07)

The generic `HttpActor` boundary now rejects `websocket` and
`websocket-cached` requests instead of silently routing them through HTTP.
`sse` and `auto` retain the existing HTTP path. This is an explicit event/
request outcome and prevents a false Pi-parity claim until the provider-scoped
WebSocket adapter exists; a focused async test pins the rejection contract.

## Deferred capability boundary (2026-08-07)

`ProviderActor` now exposes owned `FetchDeferred` and `CancelDeferred` mailbox
commands. `StreamFn` supplies explicit default unsupported errors for adapters
without these capabilities. Successful deferred fetches use the same
actor-owned, joined stream pump as normal requests; no task is detached and
generic HTTP is not reinterpreted as deferred-provider behavior.

The runtime YAML state oracle now also asserts the final assistant's deferred
stop reason and complete handle fields. `deferred-response.yaml` therefore
proves the full event sequence → actor snapshot contract, not just wire
serialization or event cardinality.

## Current-state reconciliation (2026-08-08)

The generic `StreamFn`/`ProviderActor` seam is the intended injection point for
the missing Codex adapter; no generic WebSocket implementation belongs in
`HttpActor`. The adapter contract must supply an owned socket constructor,
Codex Responses envelope/decoder, continuation-cache key, fallback decision,
and close/cleanup events. Until those provider-specific facts are implemented,
the explicit unsupported result is the truthful behavior and is covered by
`default_http_boundary_rejects_unsupported_websocket_transport`.

## Provider-scoped capability slice (2026-08-08)

`runie-core` now exposes `WebSocketAdapter` beside `StreamFn` and
`ProviderActor::new_with_websocket` as the owned injection boundary. A
WebSocket request is routed to the injected adapter only; absent an adapter it
still returns the explicit unsupported error, and the generic HTTP actor never
interprets the request. `websocket_transport_uses_only_the_injected_provider_adapter`
proves this with a provider stream that deliberately errors on ordinary SSE.
The Codex adapter's actual socket/envelope/decoder/fallback implementation
remains provider-specific work behind this seam.

## Codex continuation/fallback policy increment (2026-08-09)

`provider::codex` now exposes pure provider-owned continuation-frame
construction and source-aligned WebSocket failure decisions: one retry for a
missing continuation, one pre-stream connection-limit retry, SSE fallback only
for other pre-stream transport failures, and propagation after stream start.
Deterministic unit coverage pins these decisions. Socket acquisition, cached
connection ownership, and actual fallback execution remain in the concrete
adapter boundary.

Session cache increment (2026-08-09): `provider::codex` now defines typed
session/account cache state for continuation response IDs and per-session SSE
fallback markers. `clear_session` removes every account continuation and its
fallback marker, while `clear` provides the global cleanup boundary. The cache
does not own socket handles yet; the concrete adapter must close those handles
before invoking cleanup.

Injected socket adapter increment (2026-08-09): `CodexWebSocketAdapter` now
owns the provider-scoped request lifecycle behind `CodexWebSocketConnector` and
`CodexWebSocket`. It resolves the Codex URL, adds the Responses WebSocket beta
header, sends a `response.create` frame, validates/collects object messages
through the existing Responses decoder, and closes the socket on success,
send/receive failure, malformed frames, and normal EOF. Connection/send
failures can use an explicitly injected ordinary `StreamFn` fallback. The
connector remains injected so production networking and replay remain separate;
the adapter itself no longer pretends generic HTTP is WebSocket transport.

Production connector increment (2026-08-09): `TokioCodexWebSocketConnector`
now provides the live connector implementation with `tokio-tungstenite`. It
builds the header-bearing handshake request, enforces
`websocketConnectTimeoutMs`, translates text/ping/close frames, rejects binary
frames, and owns close/error conversion. The connector is still selected by
the application through the provider adapter injection boundary, keeping
network side effects out of replay and generic HTTP code.

Cache integration increment (2026-08-09): `CodexWebSocketAdapter` now derives
the session/account key from the request options, attaches cached
`previous_response_id` values for `websocket-cached`/`auto` requests, stores
the terminal response id, and falls back to the injected ordinary provider
only before the WebSocket stream has started. Post-start transport/protocol
failures propagate instead of replaying partial output through SSE.

Pre-stream error increment (2026-08-09): an initial provider `error` envelope
remains outside the started-stream state, so the adapter can close the socket
and invoke its explicit fallback capability. A focused fake-socket regression
covers this boundary; post-stream failures continue to propagate.

Continuation retry increment (2026-08-09): a cached
`previous_response_not_found` error closes the stale socket, removes the
account-scoped continuation, and performs exactly one fresh connection. A
two-socket regression proves the second envelope omits the stale response ID;
repeated errors cannot recurse indefinitely.

Connection-limit retry increment (2026-08-09): an initial
`websocket_connection_limit_reached` envelope closes and reconnects once with
the same request; a second such failure is not retried. The adapter regression
covers the two owned attempts and terminal cleanup.

Deployment wiring increment (2026-08-09): `CodexWebSocketAdapter::production`
now constructs the Tokio connector and source-shaped Responses request builder;
the live binary injects it into `ProviderActor` with the existing ordinary
stream as fallback. Request options contribute authorization and provider
headers at the adapter boundary, while replay continues to use injected fake
connectors.
Fallback-state increment (2026-08-08): provider-scoped pre-stream WebSocket
failures now mark the session/account key as SSE-fallback-active inside the
adapter-owned cache. Subsequent cached requests honor that state when a
fallback capability is available, and the lifecycle regression asserts the
state transition; generic HTTP remains uninvolved.

Cleanup API increment (2026-08-08): `CodexWebSocketAdapter` now exposes
acknowledged async `clear_session` and `clear` methods over its owned cache.
They remove continuation IDs and fallback markers after socket lifecycle
settlement; generic `ProviderActor` does not mutate the cache directly.

Codex deployment and retry closure (2026-08-09): the live TUI binary now
constructs `CodexWebSocketAdapter::production` behind the provider actor's
injected WebSocket capability. Its production connector owns the handshake,
headers, frame receive, timeout, and close boundary. The adapter's bounded
pre-stream retries match Pi for connection-limit and missing-continuation
errors; post-stream transport failures propagate, and initial failures use the
explicit SSE fallback policy. Provider-specific deferred polling/decoding
remains the separate open contract.
