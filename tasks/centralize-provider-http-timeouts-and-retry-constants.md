# Centralize provider HTTP timeouts and retry constants

## Status

`todo`

## Description

`reqwest` request/connect timeouts (`120`, `10`) are duplicated in `runie-provider/src/openai/mod.rs`, `runie-provider/src/model_client.rs`, and `runie-core/src/actors/provider/factory.rs`. Status-code classification (`401`, `403`, `429`, `>=500`) is duplicated in `provider_trait.rs` and `retry.rs`. `RetryConfig` exists but `with_retry` ignores it.

## Acceptance criteria

1. **Unit tests** — All provider HTTP clients and classifiers use the same named constants; `RetryConfig` is either honored or removed.
2. **E2E tests** — Provider replay with timeout/retry fixtures still behaves correctly.
3. **Live tmux tests** — Run a provider turn in tmux; verify normal and error responses.

## Tests

### Unit tests
- Central constants exist and are used by all three client builders.
- Status classifier covers 401/403/429/5xx from both HTTP and SSE errors.

### E2E tests
- Replay fixtures with rate-limit and server-error responses retry/fail as expected.

### Live tmux tests
- Submit a prompt and observe normal streaming; trigger an auth error and verify message.
