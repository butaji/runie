# Add keyring store trait with mock

## Status

`todo`

## Context

Codex uses a `KeyringStore` trait with real and mock implementations, making credential resolution testable without mocking the OS keyring globally.

## Goal

Introduce a `KeyringStore` trait in Runie with OS and in-memory mock backends.

## Acceptance Criteria
- [ ] Define `KeyringStore` trait.
- [ ] Implement OS backend and `MockKeyringStore`.
- [ ] Use trait in `CredentialResolver` and tests.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for both backends.
- **Layer 2 — Event Handling:** Credential facts use store.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Resolver tests pass without env locks.
- **Live tmux validation:** `/login` still stores in OS keyring.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
