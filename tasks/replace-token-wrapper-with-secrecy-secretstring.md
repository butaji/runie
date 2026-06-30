# Replace Token wrapper with secrecy::SecretString

## Status

`todo`

## Context

`crates/runie-core/src/auth/storage.rs` defines a thin `Token` wrapper around `secrecy::SecretString` that adds `expose()`, `as_secret()`, `PartialEq`, and `From` impls. The wrapper adds no domain behavior and forces every caller to learn an extra type.

## Goal

Use `secrecy::SecretString` directly, or alias `Token` to `SecretString`, and delete the custom wrapper code.

**Design impact:** No change to TUI element design or composition. Only internal credential-handling behavior changes.

## Acceptance Criteria

- [ ] Remove the `Token` struct and its impls from `storage.rs`.
- [ ] Update all call sites to use `secrecy::SecretString` and `ExposeSecret`.
- [ ] Keep secret redaction in `Debug` output.
- [ ] Ensure no plaintext `String` is introduced in the refactor.

## Tests

- **Layer 1 — State/Logic:** Unit test that secrets do not appear in `Debug` formatting.
- **Layer 1:** Round-trip a credential through resolver → keyring/config without exposing plaintext.
- **Layer 2 — Event Handling:** `ConfigLoaded` carrying a provider key is handled without exposing the key.
- **Layer 3 — Rendering (if TUI-visible):** Snapshot of `/inspect` or settings masks the API key.
- **Layer 4 — E2E:** Headless provider resolution succeeds with a key loaded from env/keyring.
- **Live tmux validation:** Run `/login mock`, `/inspect`; confirm the API key is masked in output.
