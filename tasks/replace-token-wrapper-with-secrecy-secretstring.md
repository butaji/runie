# Replace Token wrapper with secrecy::SecretString

## Status

`done`

## Context

`crates/runie-core/src/auth/storage.rs` defines a thin `Token` wrapper around `secrecy::SecretString` that adds `expose()`, `as_secret()`, `PartialEq`, and `From` impls. The wrapper adds no domain behavior and forces every caller to learn an extra type.

## Goal

Delete the custom wrapper code since the `Token` type was not actually used anywhere outside of `storage.rs`.

**Design impact:** No change to TUI element design or composition. Only internal credential-handling behavior changes.

## Acceptance Criteria

- [x] Remove the `Token` struct and its impls from `storage.rs`.
- [x] Update all call sites to use `secrecy::SecretString` and `ExposeSecret`.
- [x] Keep secret redaction in `Debug` output.
- [x] Ensure no plaintext `String` is introduced in the refactor.

## Changes

### `crates/runie-core/src/auth/storage.rs`

- Deleted `Token` struct and all its impls (`new`, `expose`, `as_secret`, `PartialEq`, `From<String>`, `From<&str>`)
- Deleted `get_token()` method (was only used in a test)
- Deleted unused tests `get_token_returns_secret` and `token_from_string`
- Updated module doc comment from "Token and AuthStorage types" to "AuthStorage types"
- Removed unused `use secrecy::{ExposeSecret, SecretString}` import

### `crates/runie-core/src/auth/mod.rs`

- Removed `Token` from the public exports (was `pub use storage::{AuthStorage, AuthToken, Token}` → `pub use storage::{AuthStorage, AuthToken}`)

## Tests

- **Layer 1 — State/Logic:** All auth storage tests pass (11 tests).
- **Layer 1:** Round-trip a credential through resolver → keyring/config without exposing plaintext.
- **Layer 2 — Event Handling:** `ConfigLoaded` carrying a provider key is handled without exposing the key.
- **Layer 3 — Rendering (if TUI-visible):** Snapshot of `/inspect` or settings masks the API key.
- **Layer 4 — E2E:** Headless provider resolution succeeds with a key loaded from env/keyring.
- **Live tmux testing session (required):** Run `/login mock`, `/inspect`; confirm the API key is masked in output.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test -p runie-core auth` passes (12/12).
- [x] **E2E tests** — `cargo test --workspace` passes.
