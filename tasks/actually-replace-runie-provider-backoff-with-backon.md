# Actually replace `runie-provider` custom backoff with `backon`

**Status**: todo
**Milestone**: R6
**Category**: Provider / Network
**Priority": P2

**Depends on**: replace-custom-retry-with-backon
**Blocks**: none

## Description

`replace-custom-retry-with-backon.md` is marked done, but `crates/runie-provider/src/retry.rs` still contains a hand-rolled exponential backoff loop with `tokio::time::sleep`. `backon` is already in the workspace but unused. Replace the custom loop with `backon::Retryable`.

## Acceptance Criteria

- [ ] Replace `with_retry` in `retry.rs` with `backon::Retryable` and `ExponentialBuilder`.
- [ ] Use `.when()` to retry only retryable errors (transient HTTP, rate limit).
- [ ] Delete the manual sleep loop.
- [ ] Apply the same retry to `validate_api_key`/`fetch_models` if it is not already covered by stream backoff.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `backon_retries_retryable_error` — a retryable error triggers multiple attempts.
- [ ] `backon_does_not_retry_fatal_error` — a fatal error fails immediately.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `validate_key_retries_then_succeeds` — API key validation retries transient failures.

## Files touched

- `crates/runie-provider/src/retry.rs`
- `crates/runie-provider/src/lib.rs`
- `crates/runie-provider/Cargo.toml` (ensure `backon` is a normal dep)

## Notes

- `ctx7` for `backon` confirms usage: `fetch.retry(ExponentialBuilder::default()).when(...).await`.
- If `backon` remains unused after this, drop it from workspace deps.
