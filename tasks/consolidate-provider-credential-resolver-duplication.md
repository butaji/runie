# Consolidate provider credential resolver duplication

## Status

`todo`

## Context

Provider credential resolution is split across `crates/runie-core/src/auth/credential.rs` (manual `.env` re-parse + env/dotenv/keyring/config priority), `crates/runie-core/src/provider/config.rs` (extra keyring probe), `crates/runie-provider/src/lib.rs`, and `crates/runie-provider/src/factory.rs`. The same priority logic is implemented in multiple places.

## Goal

Create a single `CredentialResolver` in `runie-core` used by the provider factory, config persistence, and CLI. Use `figment::Env` / `envy` for env/dotenv layering; keep keyring and file fallbacks.

## Acceptance Criteria

- [ ] One resolver implementation with clear precedence: env → dotenv → keyring → config file.
- [ ] Remove the `.env` line-by-line re-parser in `credential.rs`.
- [ ] Remove the extra keyring probe in `provider/config.rs`.
- [ ] Provider factory and config actor use the same resolver.
- [ ] All credential tests pass.

## Design Impact

No change to TUI element design or composition. Only credential resolution behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for each precedence level and fallback.
- **Layer 2 — Event Handling:** `ConfigLoaded` / `AuthProvidersLoaded` contain the resolved key.
- **Layer 3 — Rendering:** `/inspect` masks keys as before.
- **Layer 4 — E2E:** Headless CLI loads provider key from env/keyring/config in correct order.
- **Live tmux validation:** `/login mock`, `/model mock-model`, and a headless run all resolve the same key.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
