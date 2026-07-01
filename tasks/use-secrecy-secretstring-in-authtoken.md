# Use secrecy::SecretString in AuthToken

## Status

`todo`

## Context

`crates/runie-core/src/auth/storage.rs:7-11` stores `AuthToken.token` as a plain `String`. The `secrecy` crate is declared but unused. The previous `Token` wrapper was removed without completing the migration.

## Goal

Make `AuthToken.token` a `secrecy::SecretString`. Expose plaintext only at the HTTP boundary via `ExposeSecret`. Keep `Debug` redacted.

## Acceptance Criteria

- [ ] Change `AuthToken.token` to `SecretString`.
- [ ] Update JSON/TOML serialization to handle redaction.
- [ ] Update provider HTTP header construction to use `ExposeSecret`.
- [ ] Verify keys do not leak in `Debug` or snapshots.

## Design Impact

No change to TUI element design or composition. Only credential handling behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests verify `Debug` redaction and exposure only at boundary.
- **Layer 2 — Event Handling:** Auth-loaded facts carry redacted tokens.
- **Layer 3 — Rendering:** `/inspect` masks keys.
- **Layer 4 — E2E:** Provider request includes the correct key header.
- **Live tmux validation:** `/login mock` and keyring config do not expose plaintext.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
