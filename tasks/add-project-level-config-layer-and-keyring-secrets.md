# Add project-level config layer and keyring secrets

## Status

`todo`

## Context

Config is loaded only from `~/.runie/config.toml`; API keys stored in plain TOML.

## Goal

Add project-level `.runie/config.toml` layer and move API keys to keyring-backed secrets.

## Acceptance Criteria
- [ ] Load system → user → project → env → CLI layers.
- [ ] Store keys via `keyring` with fallback to env.
- [ ] Use `figment` for merge precedence.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for layer precedence.
- **Layer 2 — Event Handling:** Config-loaded facts reflect merged layers.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Headless CLI uses project-level config in a temp dir.
- **Live tmux validation:** `/login` stores key in keyring.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
