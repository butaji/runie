# Round 2 — Provider Abstraction & Network / Retry

## Findings

### 1. Duplicated HTTP status / retry classification

- `crates/runie-core/src/provider/provider_trait.rs:120-145` — `ProviderError::from_reqwest` maps HTTP statuses to typed errors.
- `crates/runie-provider/src/retry.rs:14-53` — `from_sse_error` repeats the same mapping for SSE transport errors.

This logic should live in one place.

### 2. `RetryConfig` is defined but ignored

- `crates/runie-core/src/provider/provider_trait.rs:245-284` — `RetryConfig` is part of `ProviderMetadata`.
- `crates/runie-provider/src/retry.rs:76-84` — `with_retry` always uses `backon::ExponentialBuilder::default()`.

Either pass `RetryConfig` into `with_retry` or remove it from metadata.

### 3. `ProviderProtocol` is dead weight

- `crates/runie-provider/src/protocol.rs:18-69` — `ProviderProtocol` trait is not used polymorphically.
- `crates/runie-provider/src/lib.rs:101` — `build_provider` always calls `build_openai_provider`.
- `crates/runie-provider/src/openai/mod.rs:114-145` — `OpenAiProvider` hardcodes `openai_stream`.

Either make the protocol real or delete it.

### 4. Duplicated HTTP client creation & normalization

- `crates/runie-provider/src/model_client.rs:38-54`
- `crates/runie-provider/src/openai/mod.rs:30-45`
- `crates/runie-provider/src/lib.rs:57-82`

All build `reqwest::Client`, strip trailing slashes, and trim API keys separately. Centralize these helpers.

### 5. SSE parsing / replay logic is fragmented

- `crates/runie-provider/src/openai/stream.rs:165-189` — `parse_sse_event`
- `crates/runie-provider/src/openai/stream.rs:192-223` — `replay_sse`
- `crates/runie-provider/src/openai/protocol.rs:63-77` — `OpenAiFrame::from_line`

These all parse the same `data: {...}` / `data: [DONE]` grammar. Consolidate on `OpenAiFrame::from_line`.

### 6. Error-body parsing is fragile

- `crates/runie-provider/src/openai/types.rs:85-153` — `ErrorBodyJson` uses optional fields and accessor methods. A `#[serde(untagged)]` enum would be explicit.

## Recommended changes

1. Make `ProviderError::from_reqwest` the single classifier; implement SSE error conversion by extracting the inner `reqwest::Error`.
2. Use `RetryConfig` in `with_retry`, or remove it from metadata.
3. Delete `ProviderProtocol` if it remains unused; otherwise make `OpenAiProvider` generic over it.
4. Centralize `reqwest::Client` creation and URL/key normalization in `runie-provider::http`.
5. Unify SSE parsing/replay on `OpenAiFrame::from_line`.
6. Use `#[serde(untagged)]` for provider error-body shapes.

## Task mapping

| Finding | Task file | Status |
|---------|-----------|--------|
| Centralize HTTP status classification | `tasks/centralize-provider-error-status-classification.md` | **new** |
| Use `RetryConfig` or remove it | `tasks/use-retryconfig-in-with-retry-or-remove-it.md` | **new** |
| Remove or real-ify `ProviderProtocol` | `tasks/remove-or-make-real-providerprotocol-abstraction.md` | **new** |
| Centralize HTTP client & normalization | `tasks/centralize-reqwest-client-and-url-normalization.md` | **new** |
| Unify SSE parsing on `OpenAiFrame` | `tasks/unify-sse-parsing-on-openai-frame.md` | **new** |
| Use untagged enum for error bodies | `tasks/use-untagged-enum-for-provider-error-bodies.md` | **new** |
