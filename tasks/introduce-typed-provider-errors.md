# Introduce typed provider errors

## Status

`todo`

## Context

Provider errors collapse to `anyhow`; retry logic string-matches for `rate limit` / `timeout`.

## Goal

Replace with a `ProviderError` enum (Auth, RateLimit, ContextLength, Server, Network, etc.) and deterministic retry classification.

## Acceptance Criteria
- [ ] Define `ProviderError` with source chains.
- [ ] Map `reqwest`/`SSE` errors to variants.
- [ ] Retry logic matches on enum.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for error classification and retryability.
- **Layer 2 — Event Handling:** Error facts carry typed tag.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Replay tests for rate-limit and network errors pass.
- **Live tmux validation:** User sees clear error messages for auth/rate-limit.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
