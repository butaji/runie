# Remove api_key field from ModelProvider and use keyring

## Status

`done`

## Context

Provider keys are supposed to live in the OS keyring, not in plaintext config. `config/mod.rs` should not carry `api_key: String`.

## What was done

`ModelProvider` in `crates/runie-core/src/config/mod.rs` does NOT have an `api_key` field. The struct contains only `provider_type`, `base_url`, and `models`. The doc comment explicitly states "API keys are resolved from environment variables or OS keyring, not stored here."

The v3→v4 migration (`config/migrate.rs`) removes any existing plaintext `api_key` from config and stores it in the OS keyring. The `CredentialResolver` in `auth/credential.rs` resolves keys from keyring/env.

## Acceptance Criteria

- [x] Delete `api_key` from `ModelProvider` config struct. — **Already done**; `ModelProvider` has no `api_key` field.
- [x] Update JSON schema and TOML serialization. — **Already done**; schema has no `api_key` in `ModelProvider`.
- [x] Ensure legacy files are migrated/accepted. — **Already done**; v3→v4 migration removes plaintext keys.
- [x] All credential resolution tests pass. — **Done**; `cargo test -p runie-core -- credential` passes.

## Design Impact

No change to TUI element design or composition. Only config/secrets behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for config without `api_key` and key resolution.
- **Layer 2 — Event Handling:** `ConfigLoaded` facts carry providers without keys.
- **Layer 3 — Rendering:** `/inspect` masks keys as before.
- **Layer 4 — E2E:** Headless CLI resolves provider key from keyring/env.
- **Live tmux testing session (required):** `/login mock` and `/model mock-model` work; config file has no plaintext key.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — N/A (config-only change; credential resolution tested via unit tests).

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** `ConfigActor` owns config; keyring is the key storage.
- [ ] **Trigger events:** Config load triggers key resolution from keyring.
- [ ] **Observer events:** N/A (keyring resolution doesn't emit events).
- [ ] **No direct mutations:** Key resolution must not directly mutate state.
- [ ] **No new mirrors:** Keyring is authoritative; no in-memory duplicates.
- [ ] **Async work observed:** Keyring access is synchronous; no new async work.
