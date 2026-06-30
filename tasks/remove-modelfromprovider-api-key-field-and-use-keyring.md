# Remove api_key field from ModelProvider and use keyring

## Status

`todo`

## Context

Provider keys are supposed to live in the OS keyring, but `crates/runie-core/src/config/mod.rs:57` still carries `api_key: String`. The field is a trap for accidental plaintext storage and complicates the schema.

## Goal

Remove `api_key` from `ModelProvider`; resolve keys only through keyring/env at load time.

## Acceptance Criteria

- [ ] Delete `api_key` from `ModelProvider` config struct.
- [ ] Update JSON schema and TOML serialization.
- [ ] Ensure legacy files are migrated/accepted.
- [ ] All credential resolution tests pass.

## Design Impact

No change to TUI element design or composition. Only config/secrets behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for config without `api_key` and key resolution.
- **Layer 2 — Event Handling:** `ConfigLoaded` facts carry providers without keys.
- **Layer 3 — Rendering:** `/inspect` masks keys as before.
- **Layer 4 — E2E:** Headless CLI resolves provider key from keyring/env.
- **Live tmux validation:** `/login mock` and `/model mock-model` work; config file has no plaintext key.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
