# Unify file picker and panel list widget

## Status

`todo`

## Context

The `@` file picker duplicates the same custom list-building logic as `popups/panel/list.rs`.

## Goal

Extract a shared `ratatui::widgets::List`-based helper used by both.

## Acceptance Criteria
- [ ] Shared list rendering helper.
- [ ] File picker preserves `/` suffix and max height.
- [ ] Panel list behavior unchanged.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** Snapshots for panel and file picker unchanged.
- **Layer 4 — E2E:** N/A.
- **Live tmux validation:** `@` file picker and `/` command palette render consistently.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
