# Actually replace `runie-provider` custom backoff with `backon`

**Status**: done
**Milestone**: R6
**Category**: Provider / Network
**Priority**: P2

**Depends on**: replace-custom-retry-with-backon
**Blocks**: none

## Description

`replace-custom-retry-with-backon.md` is marked done, but `crates/runie-provider/src/retry.rs` still contains a hand-rolled exponential backoff loop with `tokio::time::sleep`. `backon` is already in the workspace but unused. Replace the custom loop with `backon::Retryable`.

## Acceptance Criteria

- [x] Replace `with_retry` in `retry.rs` with `backon::Retryable` and `ExponentialBuilder`.
- [x] Use `.when()` to retry only retryable errors (transient HTTP, rate limit).
- [x] Delete the manual sleep loop.
- [x] Apply the same retry to `validate_api_key`/`fetch_models` if it is not already covered by stream backoff.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `backon_retries_retryable_error` — a retryable error triggers multiple attempts.
- [x] `backon_does_not_retry_fatal_error` — a fatal error fails immediately.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A — existing tests cover retry behavior.

## Files touched

- `crates/runie-provider/src/retry.rs` — already uses `backon::Retryable` with `ExponentialBuilder`

## Notes

- The retry module was already updated to use `backon` with exponential backoff.
- `is_retryable()` function determines which errors trigger retries.
