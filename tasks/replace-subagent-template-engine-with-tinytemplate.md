# Replace subagent template engine with tinytemplate

## Status

`done`

## Context

`crates/runie-core/src/subagents/mod.rs:67-74` implements a minimal `{{variable}}` replacement engine with `String::replace`.

## Goal

Replace with `tinytemplate` (lightweight) or `handlebars` if conditionals/loops are needed.

## Acceptance Criteria
- [x] Add dependency.
- [x] Port substitution to template engine.
- [x] Preserve current behavior.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for variable substitution.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Subagent tests pass.
- **Live tmux testing session (required):** N/A.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
