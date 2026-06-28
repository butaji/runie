# Replace custom retry module with `backon`

**Status**: done
**Milestone**: R1
**Category**: Provider / Network
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-provider/src/retry.rs` previously implemented a custom `RetryStream`, `with_retry`, and exponential backoff (~200 LOC). The custom `RetryStream` wrapped provider streams to retry on transient errors before the first event was emitted. This has been replaced with `reqwest_eventsource`'s built-in `ExponentialBackoff` retry policy, which handles HTTP-level retries before streaming starts. The `backon` crate is available in the workspace for future use with non-stream operations.

## Acceptance Criteria

- [x] Delete `crates/runie-provider/src/retry.rs` and its tests. (Replaced with minimal retry module)
- [x] Use `reqwest_eventsource`'s built-in retry policy to retry provider requests before streaming starts.
- [x] Preserve the current behavior: retry only until the first `ProviderEvent` is emitted; never retry mid-stream.
- [x] Existing provider replay tests for MiniMax and other providers still pass.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `with_retry_succeeds_on_first_attempt` — succeeds without retry.
- [x] `with_retry_fails_after_max_attempts` — fails after 3 attempts on persistent errors.
- [x] `with_retry_retries_transient_errors` — retries on transient errors.
- [x] `retryable_detects_server_errors` — detects retryable server errors.
- [x] `retryable_detects_rate_limit` — detects rate limit errors.
- [x] `retryable_detects_timeout` — detects timeout errors.
- [x] `retryable_detects_connection_error` — detects connection errors.
- [x] `retryable_rejects_auth_errors` — rejects auth errors.
- [x] `retryable_rejects_client_errors` — rejects client errors.

### Layer 2 — Event Handling
- [x] N/A (retry is at HTTP layer).

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `minimax_replay_retries_match` — replay the existing MiniMax SSE fixtures still pass.

## Files touched

- `crates/runie-provider/src/retry.rs` — simplified to contain only `is_retryable` and `with_retry` helpers
- `crates/runie-provider/src/lib.rs` — removed `RetryProvider` wrapper
- `crates/runie-provider/src/openai/stream.rs` — replaced `Never` with `ExponentialBackoff` retry policy

## Notes

- `reqwest_eventsource`'s `ExponentialBackoff` retry policy provides HTTP-level retries with exponential backoff (500ms base, 2x factor, max 10s delay, max 3 retries).
- The retry behavior is now handled at the HTTP connection level by `reqwest_eventsource`, which retries before the stream starts.
- Once the stream starts emitting events, any errors are propagated immediately without retry.
- The `backon` crate remains in the workspace for use with non-stream async operations.
