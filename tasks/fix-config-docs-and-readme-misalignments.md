# Fix config docs and README misalignments

## Status

`todo`

## Context

Multiple docs/schema/CLI misalignments: `provider_type` vs `type`, missing `base_url`, no permissions/env/keyring docs, wrong `justfile` binary name, TUI clap name, README print example, README modes table, Architecture.md MCP `--transport` example.

## Goal

Fix all listed misalignments in one pass.

## Acceptance Criteria
- [ ] Use `type = "..."` in Configuration.md provider blocks.
- [ ] Add `base_url` or make schema optional.
- [ ] Add permissions and env/keyring sections.
- [ ] Fix `justfile` and TUI clap name.
- [ ] Fix README examples and modes table.
- [ ] Remove unsupported `--transport stdio` from Architecture.md.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** `cargo run --bin runie-tui -- --help` and `just tui` work.
- **Live tmux testing session (required):** N/A.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
