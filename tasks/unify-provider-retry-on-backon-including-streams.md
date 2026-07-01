# Unify provider retry on backon including streams

## Status

`done`

**Completed:** 2026-07-01

## Context

API-key validation uses workspace `backon`, but streaming used `reqwest_eventsource::retry::ExponentialBackoff` (`provider/src/openai/stream.rs:87-96`). Two different retry policies and implementations.

## Goal

Unify retry on `backon`: retry the stream establishment request with `backon`, then surface stream errors immediately once SSE bytes start flowing.

## Changes Made

### `crates/runie-provider/src/openai/stream.rs`

**Removed:**
- `configure_backoff()` function that configured `ExponentialBackoff` on `EventSource`
- `build_eventsource()` sync helper (dead code after refactor)
- `use reqwest_eventsource::retry::ExponentialBackoff` import

**Added:**
- `use backon::{ExponentialBuilder, Retryable}` import
- `use reqwest_eventsource::retry::Never` import
- `async fn build_eventsource_with_retry()` — wraps `EventSource::new()` in `backon` retry with `ExponentialBuilder::default()`, using `crate::retry::is_retryable()` to determine which errors are retryable. Sets `set_retry_policy(Never)` on the EventSource to disable internal retry.
- Updated `openai_event_stream()` to call `build_eventsource_with_retry().await` instead of `build_eventsource()` + `configure_backoff()`

**Also improved `parse_sse_result()`** to use `crate::retry::from_sse_error()` for typed error classification instead of a raw string format.

### `crates/runie-provider/src/lib.rs`

Updated comment from "Retries are handled by reqwest_eventsource's ExponentialBackoff policy" to "Retries are handled by backon for stream establishment (see stream.rs)".

### `crates/runie-provider/src/retry.rs`

Enhanced `is_retryable()` to use `ProviderError::is_retryable()` via downcast as the primary path, falling back to `ProviderError::from_reqwest()` for `reqwest::Error`. Added `from_sse_error()` function that classifies `reqwest_eventsource::Error` variants into typed `ProviderError`. Added Layer 1 tests for typed error retryability.

## Acceptance Criteria

- [x] Remove `reqwest_eventsource` backoff configuration.
- [x] Use `backon` for stream-establishment retries.
- [x] Keep "no byte-level retry once SSE starts" rule.
- [x] All provider retry tests pass.

## Design Impact

No change to TUI element design or composition. Only provider retry behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for retry predicates and backoff (17 retry tests pass).
- **Layer 2 — Event Handling:** Retry events surface correctly.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Provider replay fixtures with transient failure pass.

## Completion Validation

- [x] **Unit tests** — `cargo test -p runie-provider -- retry` passes (17 tests).
- [x] **E2E tests** — `cargo test --workspace` passes.
- [x] **Live tmux run tests** — N/A (not TUI-affecting).
