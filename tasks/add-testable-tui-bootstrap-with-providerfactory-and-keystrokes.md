# Add testable TUI bootstrap with ProviderFactory and keystrokes

## Status

`todo`

## Context

`crates/runie-tui/src/main.rs` hard-codes `BuiltProviderFactory` and reads real crossterm input; there is no entry point for deterministic scenario replay.

## Goal

Extract `runie_tui::tests::run_with_backend_and_provider` accepting a `ProviderFactory`, `TestBackend`, and keystroke DSL.

## Acceptance Criteria
- [ ] Refactor main into bootstrap + run functions.
- [ ] Expose testable entry point.
- [ ] Preserve production startup path.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** Keystroke DSL produces expected events.
- **Layer 3 — Rendering:** `TestBackend` scenario snapshots pass.
- **Layer 4 — E2E:** TUI replay with fixture provider passes.
- **Live tmux testing session (required):** TUI starts normally.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
