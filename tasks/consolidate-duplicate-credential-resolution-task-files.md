# Consolidate duplicate credential resolution task files

## Status

`done`

## Context

`centralize-provider-credential-resolution.md` and `consolidate-provider-credential-resolver-duplication.md` describe nearly identical work and are both marked done.

## Goal

Archive one task file and keep one canonical tracker; update `tasks/index.json`.

## Acceptance Criteria
- [ ] Choose canonical file.
- [ ] Move duplicate content into canonical file.
- [ ] Archive or delete duplicate.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** N/A.
- **Live tmux testing session (required):** N/A.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
