# Adopt camino Utf8PathBuf for internal paths

## Status

`todo`

## Context

Codebase stores and serializes paths as `PathBuf`, then repeatedly calls `to_string_lossy()` or `to_str().unwrap()` for JSON, display, and map keys.

## Goal

Use `camino::Utf8PathBuf`/`Utf8Path` for config paths, project paths, tool paths, and trust keys; keep `std::path::PathBuf` at the OS boundary.

## Acceptance Criteria
- [ ] Add `camino` dependency.
- [ ] Migrate internal path fields and map keys.
- [ ] Update serialization.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for path round-trip.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Tool and trust tests pass.
- **Live tmux testing session (required):** File tools work.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
