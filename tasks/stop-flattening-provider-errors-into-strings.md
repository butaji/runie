# Stop flattening provider errors into strings

## Status

`todo`

## Description

`ModelError`, `ProviderEvent::Error`, and SSE error handling convert typed errors to strings, losing structure. Propagate `ProviderError`/`ModelError` and add a structured `Event::ModelError` variant.

## Acceptance criteria

1. **Unit tests** — `ProviderError`/`ModelError` survive retries, SSE parsing, and event conversion.
2. **E2E tests** — Replay fixtures with errors still classify retry vs fatal correctly.
3. **Live tmux tests** — Cause a rate-limit and an auth error; verify distinct UI behavior.

## Tests

### Unit tests
- Error propagation through provider stack.

### E2E tests
- Replay with 401/429/500 responses.

### Live tmux tests
- Use invalid/expired keys and observe error kind.

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** `ProviderActor` owns provider state; typed errors are part of it.
- [ ] **Trigger events:** Typed errors trigger retry/fatal classification.
- [ ] **Observer events:** `ModelError` event notifies observers of error condition.
- [ ] **No direct mutations:** Error propagation must not directly mutate state.
- [ ] **No new mirrors:** Typed errors are authoritative in provider; no duplicates.
- [ ] **Async work observed:** Error propagation is synchronous; retry has JoinHandle.
