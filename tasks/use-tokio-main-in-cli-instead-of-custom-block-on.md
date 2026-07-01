# Use tokio::main in CLI instead of custom block_on

## Status

`todo`

## Context

`crates/runie-cli/src/main.rs:101-119` builds a new `current_thread` tokio runtime for each async subcommand.

## Goal

Convert `main` to `#[tokio::main(flavor = "multi_thread")] async fn main()` and await subcommands directly.

## Acceptance Criteria
- [ ] Remove custom `block_on` helper.
- [ ] Update async subcommand entry points.
- [ ] `cargo check -p runie-cli` passes.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** CLI tests pass.
- **Live tmux validation:** `runie-headless print` works.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
