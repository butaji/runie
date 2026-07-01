# Route provider config=None through CredentialResolver

## Status

`todo`

## Context

`runie-provider/src/lib.rs::resolve_credentials` has a separate `config=None` branch that reads `std::env::var(&meta.env_var)` directly, bypassing dotenv/keyring/config priority.

## Goal

Route the `None` branch through `CredentialResolver` so the unified priority chain is always used.

## Acceptance Criteria
- [ ] Remove direct `std::env::var` call in provider lib.
- [ ] Use `CredentialResolver` (or `ProviderConfigResolver::env_only`) for `config=None`.
- [ ] Add Layer-1 precedence test.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for env/dotenv precedence when config is None.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Provider replay tests pass.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
