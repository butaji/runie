# Delete dead InspectReport build method

## Status

`todo`

## Context

`crates/runie-cli/src/inspect/mod.rs:113-128` `InspectReport::build()` is annotated `#[allow(dead_code)]` and unused after the CLI migrated to async `build_with_config_actor`.

## Goal

Delete `build()` and its doc comment.

## Acceptance Criteria
- [ ] Delete method.
- [ ] `cargo check -p runie-cli` passes.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** CLI tests pass.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
