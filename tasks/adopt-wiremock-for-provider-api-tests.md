# Adopt wiremock for provider API tests

## Status

`todo`

## Context

`crates/runie-provider/src/tests.rs:293-494` manually binds a TCP listener and crafts raw HTTP/1.1 responses for API-key validation tests. This is ~90 lines of bespoke server code per test.

## Goal

Use `wiremock` for HTTP mocking. Replace manual servers with `MockServer::start()`, request matchers, and `ResponseTemplate`.

## Acceptance Criteria

- [ ] Add `wiremock` as a dev-dependency.
- [ ] Migrate validation tests to `wiremock`.
- [ ] Delete manual TCP/HTTP server code.
- [ ] All provider tests pass.

## Design Impact

No change to TUI element design or composition. Only test infrastructure changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Provider validation tests pass under `cargo test -p runie-provider`.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
