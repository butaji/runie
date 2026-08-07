# p43 — Preserve provider retry metadata before `maxRetryDelayMs`

Status: planned

Pi's retry helper derives a delay from provider error metadata and clamps it
with `maxRetryDelayMs` (defaulting to Pi's provider retry policy). Runie's
current `HttpActor` retries transport failures immediately, and
`StreamError::Api` does not retain `Retry-After` or provider retry metadata.

Adding `max_retry_delay_ms` to `SimpleStreamOptions` now would be cosmetic:
there is no delay input to clamp and no provider response/error metadata to
preserve. The correct next boundary is an owned transport error carrying
retry metadata plus an injectable async delay policy. Then YAML can declare
the cap and assert observed attempts/delay decisions without sleeping in
tests.

This task is deliberately separate from p37's already-supported timeout and
retry-count options; it prevents a false 100% parity claim for retry timing.
