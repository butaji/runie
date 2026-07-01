# Inject provider factory into headless CLI and runtime

## Status

`todo`

## Context

`runie_agent::headless_helper::run_headless` and `run_headless_cli` hard-code `BuiltProviderFactory`; `HeadlessRuntime::spawn` has no seam for replay providers.

## Goal

Accept `Arc<dyn ProviderFactory>` in headless CLI/runtime so Grok fixtures can be replayed.

## Acceptance Criteria
- [ ] Add factory parameter to headless functions.
- [ ] Default to `BuiltProviderFactory` for production.
- [ ] Update CLI callers.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Headless tests with mock factory pass.
- **Live tmux testing session (required):** Headless CLI with real provider works.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
