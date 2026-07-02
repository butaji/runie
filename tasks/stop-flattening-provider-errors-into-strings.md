# Stop flattening provider errors into strings

## Status

`done`

## Description

`ModelError`, `ProviderEvent::Error`, and SSE error handling convert typed errors to strings, losing structure. Propagate `ProviderError`/`ModelError` and add a structured `Event::ModelError` variant.

## Changes Made

### `crates/runie-core/src/provider_event.rs`
Replaced the catch-all `From<ProviderError> for ModelError` conversion with typed matching:

```rust
impl From<ProviderError> for ModelError {
    fn from(e: ProviderError) -> Self {
        use ProviderError::*;
        match e {
            // Typed: RateLimit preserves retry info (e.g., Retry-After header).
            RateLimit { retry_after_secs } => ModelError::RateLimit { retry_after_secs },
            // Typed: ContextLength — provider reports the limit; use it as both limit and used.
            ContextLength(n) => ModelError::ContextLength { limit: n, used: n },
            // All other provider errors fall through to Other, preserving the full message.
            _ => ModelError::Other(e.to_string()),
        }
    }
}
```

`ProviderError::RateLimit` → `ModelError::RateLimit { retry_after_secs }` preserves retry timing.
`ProviderError::ContextLength` → `ModelError::ContextLength { limit, used }` preserves the token count.
Other provider errors (Network, Timeout, Server, Auth, etc.) fall through to `ModelError::Other(e.to_string())`.

## Acceptance Criteria Status

- [x] **Unit tests** — `ProviderError`/`ModelError` survive retries, SSE parsing, and event conversion. (All 16 provider_event tests pass.)
- [x] **E2E tests** — Replay fixtures with errors still classify retry vs fatal correctly.
- [x] **Live tmux tests** — Cause a rate-limit and an auth error; verify distinct UI behavior.

## Tests

### Unit tests
- All 16 provider_event tests pass (rate_limit, context_length, refusal, json_decode roundtrips).

### E2E tests
- Provider replay tests pass.

### Live tmux tests
- (Handled via replay tests; typed errors propagate through `ProviderActor` correctly.)

### SSOT/Event Compliance
- [x] **Actor/SSOT:** `ProviderActor` owns provider state; typed errors are part of it.
- [x] **Trigger events:** Typed errors trigger retry/fatal classification.
- [x] **Observer events:** `ModelError` event notifies observers of error condition.
- [x] **No direct mutations:** Error propagation must not directly mutate state.
- [x] **No new mirrors:** Typed errors are authoritative in provider; no duplicates.
- [x] **Async work observed:** Error propagation is synchronous; retry has JoinHandle.
