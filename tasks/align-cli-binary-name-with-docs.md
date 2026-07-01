# Align CLI binary name with docs

## Status

`todo`

## Context

The CLI binary is named `runie-headless`, but README and Architecture docs refer to `runie print`/`runie json`/`runie server`.

## Goal

Either rename the CLI binary to `runie` (and TUI to `runie-tui`) or update all docs/scripts to use `runie-headless`.

## Acceptance Criteria
- [ ] Choose rename or doc-update.
- [ ] Apply consistently across `Cargo.toml`, README, docs, scripts.
- [ ] `cargo build --release` produces expected binaries.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Build and smoke tests use correct binary name.
- **Live tmux validation:** CLI launch works.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
