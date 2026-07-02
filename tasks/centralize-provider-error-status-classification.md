# Centralize provider error status classification

## Status

`todo`

## Description

HTTP status classification exists in both `ProviderError::from_reqwest` and `from_sse_error`. Make `ProviderError::from_reqwest` the single classifier and convert SSE errors through it.

## Acceptance criteria

1. **Unit tests** — Each status code (401, 403, 429, 5xx, etc.) maps to the expected error type from both HTTP and SSE paths.
2. **E2E tests** — Provider replay with rate-limit/server-error fixtures still retries/fails correctly.
3. **Live tmux tests** — Use an invalid/expired key in tmux and confirm the error message is accurate.

## Tests

### Unit tests
- Status-code-to-error-type mapping for HTTP and SSE.

### E2E tests
- Replay fixtures containing 401, 429, 500 responses.

### Live tmux tests
- Configure a bad API key and submit a prompt; verify the error.
