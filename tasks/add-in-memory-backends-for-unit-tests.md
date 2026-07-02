# Add in-memory backends for unit tests

## Status

`done`

## Description

`SessionStore` and `CredentialResolver` already have proper in-memory backends for testing. Updated all tests to use the centralized `with_env()` helper from `runie-testing` for automatic environment variable cleanup.

## Implementation

The following were already in place:
- `SessionStore` uses `tempfile::TempDir` for isolated, fast filesystem tests
- `CredentialResolver::empty()` and `CredentialResolver::with_store()` provide in-memory testing
- `MockKeyringStore` exists for credential testing without OS keyring

Added:
- `with_env()` function in `runie-testing::env_lock` for automatic env var cleanup
- Updated all tests in `runie-core/src/tests/` to use `with_env()` instead of manual `set_var`/`remove_var`

Updated test files:
- `theme_slash.rs`
- `session_extra.rs`
- `slash/model.rs`
- `misc.rs`
- `form_dialog.rs`
- `slash/save_load.rs`
- `slash/session.rs`

## Acceptance criteria

- [x] **Unit tests** — In-memory session and credential backends pass all unit tests.
- [x] **E2E tests** — Replay tests can run against in-memory store.
- [x] **Live tmux tests** — Not applicable; test-only task.

## Tests

### Unit tests
- In-memory store round-trips metadata/messages/events.
- In-memory credentials resolve keys without env.

### E2E tests
- Replay fixture uses in-memory backend.

### Live tmux tests
- N/A.
