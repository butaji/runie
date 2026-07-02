# Add in-memory backends for unit tests

## Status

`todo`

## Description

`SessionStore` and `CredentialResolver` currently require filesystem/env. Provide in-memory backends so unit tests are isolated and fast.

## Acceptance criteria

1. **Unit tests** — In-memory session and credential backends pass all unit tests.
2. **E2E tests** — Replay tests can run against in-memory store.
3. **Live tmux tests** — Not applicable; test-only task.

## Tests

### Unit tests
- In-memory store round-trips metadata/messages/events.
- In-memory credentials resolve keys without env.

### E2E tests
- Replay fixture uses in-memory backend.

### Live tmux tests
- N/A.
