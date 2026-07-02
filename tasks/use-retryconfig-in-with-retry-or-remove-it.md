# Use `RetryConfig` in `with_retry` or remove it

## Status

`todo`

## Description

`RetryConfig` is part of `ProviderMetadata` but `with_retry` ignores it and uses `backon::ExponentialBuilder::default()`. Either wire the config in or delete it.

## Acceptance criteria

1. **Unit tests** — `with_retry` honors `RetryConfig` values, or `RetryConfig` no longer exists.
2. **E2E tests** — A replay with retryable failures retries the configured number of times.
3. **Live tmux tests** — Induce a transient provider failure in tmux and observe retry behavior.

## Tests

### Unit tests
- Retry count, initial delay, and multiplier are applied.

### E2E tests
- Replay fixture with retryable errors.

### Live tmux tests
- Temporarily block the provider endpoint and submit a prompt.
