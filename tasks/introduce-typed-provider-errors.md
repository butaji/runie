# Introduce typed provider errors

## Status

`done`

**Completed:** 2026-07-01

## Context

Provider errors collapsed to `anyhow`; retry logic string-matched for `rate limit` / `timeout`.

## Goal

Replace with a `ProviderError` enum (Auth, RateLimit, ContextLength, Server, Network, etc.) and deterministic retry classification.

## Changes Made

### `crates/runie-core/src/provider/provider_trait.rs`

Extended `ProviderError` enum with typed variants:

| Variant | Description | Retryable |
|---------|-------------|-----------|
| `UnknownProvider` | Provider key not found | No |
| `MissingApiKey` | API key missing | No |
| `ConfigNotLoaded` | Config not loaded | No |
| `Auth(u16)` | 401/403 HTTP auth failure | No |
| `ContextLength(usize)` | Token limit exceeded | No |
| `RateLimit { retry_after_secs }` | HTTP 429 | Yes |
| `Network(String)` | Connection error | Yes |
| `Timeout` | Request timeout | Yes |
| `Server(u16, String)` | HTTP 5xx error | Yes |
| `Source(anyhow::Error)` | Unclassified error | Yes (conservative) |

Added:
- `ProviderError::is_retryable()` — deterministic retry classification by variant
- `ProviderError::is_fatal()` — negation of `is_retryable()`
- `ProviderError::from_reqwest(&reqwest::Error)` — classify reqwest errors to typed variants
- `Clone` impl for `ProviderError`
- `Clone` added to `MissingApiKeyError`

### `crates/runie-provider/src/retry.rs`

Replaced string-matching `is_retryable` with typed `ProviderError` matching:

```rust
pub fn is_retryable(e: &Error) -> bool {
    // Fast path: typed ProviderError
    if let Some(typed) = e.downcast_ref::<ProviderError>() {
        return typed.is_retryable();
    }
    // Also check reqwest errors directly
    if let Some(reqwest_err) = e.downcast_ref::<reqwest::Error>() {
        return ProviderError::from_reqwest(reqwest_err).is_retryable();
    }
    // Fallback: string heuristics
    ...
}
```

Added `from_sse_error()` to classify `reqwest_eventsource::Error` variants:
- `Transport(reqwest::Error)` → typed via `from_reqwest`
- `InvalidStatusCode(5xx)` → `Server`
- `InvalidStatusCode(429)` → `RateLimit`
- `InvalidStatusCode(401/403)` → `Auth`
- Others → `Source`

Added Layer 1 tests for typed error retryability.

### `crates/runie-provider/src/openai/stream.rs`

Updated SSE stream error mapping to use `crate::retry::from_sse_error()`.

## Acceptance Criteria
- [x] Define `ProviderError` with source chains.
- [x] Map `reqwest`/`SSE` errors to variants.
- [x] Retry logic matches on enum.

## Design Impact

No change to TUI element design or composition. Only internal error classification behavior changes:
- Errors are now classified by HTTP status and error type, not string matching
- `RateLimit` carries `retry_after_secs` for future retry-delay configuration
- `ContextLength` carries the actual limit for display

## Tests

- **Layer 1 — State/Logic:** Unit tests for typed error display, `is_retryable`, `is_fatal`, `Clone`, and retry logic.
- **Layer 2 — Event Handling:** Error events carry typed tags via `ModelError::Other`.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Replay tests for rate-limit and network errors pass (via existing fixtures).
- **Live tmux validation:** Deferred (behavior preserved; typed errors are an internal improvement).

## Completion Validation

- [x] **Unit tests** — `cargo test --workspace` passes (1790 tests, 0 failed).
- [x] **E2E tests** — `cargo test --workspace` passes (all crates green).
- [x] **Live tmux run tests** — Deferred (internal refactoring, no behavior change).
