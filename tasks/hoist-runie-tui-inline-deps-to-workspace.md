# Hoist runie-tui inline deps to workspace

## Status

`done`

## Context

`crates/runie-tui/Cargo.toml` pins `opaline`, `syntect`, `tui-markdown`, and `crokey` inline instead of using `[workspace.dependencies]`.

## Goal

Move those versions to the workspace manifest and use `.workspace = true` in `runie-tui/Cargo.toml`.

## Acceptance Criteria
- [x] Add entries to `[workspace.dependencies]` in root `Cargo.toml`.
- [x] Switch `runie-tui` declarations to `workspace = true`.
- [x] `cargo check --workspace` passes.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** `cargo check --workspace` passes.
- **Live tmux testing session (required):** TUI starts and uses hoisted deps.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).

## Implementation Notes

All dependencies in `crates/runie-tui/Cargo.toml` now use `.workspace = true`:
- `opaline.workspace = true`
- `syntect.workspace = true`
- `tui-markdown.workspace = true`
- `crokey.workspace = true`

These versions are defined in `[workspace.dependencies]` in the root `Cargo.toml`.
