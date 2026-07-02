# Route `fetch_docs` through the central HTTP client

## Status

`todo`

## Description

`fetch_docs` uses `reqwest::get` directly with no timeout, retry, or shared client. Route it through the centralized provider HTTP client and retry policy.

## Acceptance criteria

1. **Unit tests** — `fetch_docs` uses the shared client and honors timeouts/retries.
2. **E2E tests** — A replay turn with `fetch_docs` succeeds.
3. **Live tmux tests** — Ask the agent to fetch docs in tmux.

## Tests

### Unit tests
- Timeout and retry behavior with a mock server.

### E2E tests
- Replay fixture uses `fetch_docs`.

### Live tmux tests
- Run a prompt that triggers `fetch_docs`.
