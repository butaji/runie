# Route `fetch_docs` through the central HTTP client

## Status

`done`

## Implementation

Updated `crates/runie-agent/src/tool/fetch_docs.rs` to use `runie_provider::http::build_client()` instead of direct `reqwest::get()` calls.

The `fetch_docs` function now:
1. Creates an HTTP client via `build_client()` which has consistent timeout configuration
2. Uses `client.get(url).send()` instead of `reqwest::get(url)`
3. Benefits from connection pooling via the shared client

## Context

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
