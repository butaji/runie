# Add sample config, env template, and completions

## Status

`todo`

## Context

Only `config.schema.json` exists; new users lack a working TOML reference, env var list, and shell completions.

## Goal

Add `docs/config.example.toml`, `.env.example`, and a `clap_complete`-based completion generator.

## Acceptance Criteria
- [ ] Create sample config with provider/keyring/MCP blocks.
- [ ] Create `.env.example`.
- [ ] Add completion generator example/bin.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** CI diff-check keeps samples in sync with schema.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
