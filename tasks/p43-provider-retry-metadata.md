# p43 — Preserve provider retry metadata before `maxRetryDelayMs`

Status: in progress

Pi's retry helper derives a delay from provider error metadata and clamps it
with `maxRetryDelayMs` (defaulting to Pi's provider retry policy). Runie's
current `HttpActor` retries transport failures immediately, and
`StreamError::Api` does not retain `Retry-After` or provider retry metadata.

The authoritative Pi policy is `packages/ai/src/utils/provider-retry.ts`:
retryable statuses include 408, 409, 429, and 5xx (unless provider headers
override the decision); `retry-after-ms` and `retry-after` take precedence,
then exponential backoff is used. Pi also makes the delay abortable.

Adding `max_retry_delay_ms` to `SimpleStreamOptions` now would be cosmetic:
there is no delay input to clamp and no provider response/error metadata to
preserve. The correct next boundary is an owned transport error carrying
retry metadata plus an injectable async delay policy. Then YAML can declare
the cap and assert observed attempts/delay decisions without sleeping in
tests.

This task is deliberately separate from p37's already-supported timeout and
retry-count options; it prevents a false 100% parity claim for retry timing.

## First implementation slice (2026-08-06)

`StreamError::Provider` now preserves the optional HTTP status and all response
headers through the transport boundary. Replay HTTP failures use this typed
error instead of flattening metadata into `StreamError::Api(String)`.

`provider_retry_delay_ms` is a pure policy function covering Pi's status/header
override rules, `retry-after-ms`, numeric `retry-after`, exponential fallback,
and the default/caller retry-delay cap. Its tests use no timers. The actual
abortable delay is now wired into the retry loop. `SimpleStreamOptions` exposes
`max_retry_delay_ms`, YAML fixtures can declare and assert it, and the delay
observes the actor-owned abort watch. Provider errors are retried only when the
typed status/header policy allows it; legacy network errors retain the existing
immediate retry behavior. HTTP-date `Retry-After` values are now parsed, and
`SimpleStreamOptions::retry_delay` allows replay tests to record delay decisions
without sleeping. `RetryJitterHook` now supplies Pi's bounded random jitter
explicitly: production uses a random source while replay tests assert exact
lower and upper policy edges without nondeterminism. The implementation slices
in this task are complete; a future provider-specific audit may still add
SDK-specific metadata if a concrete Pi provider exposes it.
