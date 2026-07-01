# Share configured reqwest client for provider

## Status

`done`

**Completed:** 2026-07-01

## Context

`validate_api_key` creates a new `reqwest::Client` per call; `OpenAiProvider` owns its own client. No connection reuse, proxy, or custom TLS config.

## Goal

Inject/share one configured `reqwest::Client` with timeout, user-agent, TLS, and proxy settings.

## Acceptance Criteria
- [ ] Add client factory or singleton.
- [ ] Expose optional TLS cert/proxy config.
- [ ] All provider tests pass.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Provider validation and replay tests pass; wiremock tests still work.
- **Live tmux validation:** Real MiniMax request succeeds.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
