# p37 — Pi provider transport boundary

Status: in progress

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

The YAML runner exposes these effective options at runtime; `visual-hey.yaml`
now declares and asserts `session_id`, thinking budgets, and sampling
parameters.

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
- named temperature/max-tokens/cache-retention fields: Runie's current
  provider boundary does not construct provider-specific payloads, so adding
  them to `SimpleStreamOptions` would create cosmetic parity only.

## Next implementation slice

The owned `HttpRequest` now carries body, headers, environment, metadata,
transport, and cancellation into `HttpActor`. Add deterministic clock and
backoff policy as injected dependencies, then promote each Pi option only when
one concrete adapter consumes it and a YAML event sequence asserts the
effective request. Keep unsupported provider-specific options explicitly
classified until that boundary exists.

## Verification contract

Each promoted option requires a pure option reducer/merge test, a transport
observation test, and a YAML replay assertion. Tests must use event sequences,
must not sleep, and must preserve actor ownership of mutable request state.
