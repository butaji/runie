# Plumb KeyringStore into BuiltProviderFactory

## Status

`todo`

## Context

`KeyringStore` trait and `MockKeyringStore` landed, but `ProviderConfigResolver::new` always constructs `CredentialResolver::new()` using `OsKeyringStore`. `BuiltProviderFactory` cannot inject a mock keyring, so headless tests may hit the OS keyring.

## Goal

Extend `ProviderFactory`/`BuiltProviderFactory` to accept an optional `Arc<dyn KeyringStore>` and plumb it to `ProviderConfigResolver`.

## Acceptance Criteria
- [ ] Add optional keyring store parameter to factory.
- [ ] Default to `OsKeyringStore` in production.
- [ ] Use `MockKeyringStore` in tests.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for mock keyring injection.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Provider tests pass without env locks.
- **Live tmux validation:** `/login` still uses OS keyring.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
