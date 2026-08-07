# p37 — Pi provider transport boundary

Status: in progress (HTTP/replay boundary complete; WebSocket adapter is a planned provider-specific slice)

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
- transport selection is now typed and carried through `HttpRequest`; concrete
  WebSocket adapters are still unsupported, so selecting one is observable but
  cannot yet open a WebSocket.
- nullable request headers: additive `HttpRequest` delivery now carries
  optional headers; the default adapter preserves existing body-only actors.
- `maxRetryDelayMs`: retry delay is capped by the option, with provider
  `retry-after` metadata taking precedence before the cap and injectable
  delay/jitter hooks keeping replay deterministic.
- provider-specific temperature/cache marker generation: the generic actor
  now carries both options, but concrete payload mapping still requires an
  adapter contract and must not be inferred from transport metadata alone.
- WebSocket open-handshake timeout is carried as separate request metadata;
  the current HTTP actor still does not open WebSockets.

## Next implementation slice

The owned `HttpRequest` now carries body, headers, environment, metadata,
transport, and cancellation into `HttpActor`. Add deterministic clock and
backoff policy as injected dependencies, then promote each Pi option only when
one concrete adapter consumes it and a YAML event sequence asserts the
effective request. Keep unsupported provider-specific options explicitly
classified until that boundary exists.

The YAML fixture asserts `websocket_connect_timeout_ms: 2500`, preserving this
Pi option for a future WebSocket adapter without claiming behavioral support.

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

Wire metadata preservation (2026-08-07): the generic `WireMessage::ToolResult`
projection now carries Pi `details`, `usage`, and `added_tool_names` instead
of dropping them during `default_convert_to_llm`. The reverse loop projection
restores the same fields, and the core round-trip test asserts all three. This
keeps provider-specific deferred-tool encoding possible without coupling the
generic converter to one provider protocol.

Provider lifecycle increment (2026-08-07): `ProviderActor` now aborts any
previous owned pump before acknowledging a new `Start`. This matches the
one-in-flight Pi turn contract and prevents superseded streams from publishing
events concurrently. The actor test suite covers both explicit cancellation
and replacement by a new start; no detached task or timing sleep is used.

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
