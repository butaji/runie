# Hoist runie-tui inline deps to workspace

## Status

`todo`

## Context

`crates/runie-tui/Cargo.toml` pins `opaline`, `syntect`, `tui-markdown`, and `crokey` inline instead of using `[workspace.dependencies]`.

## Goal

Move those versions to the workspace manifest and use `.workspace = true` in `runie-tui/Cargo.toml`.

## Acceptance Criteria
- [ ] Add entries to `[workspace.dependencies]` in root `Cargo.toml`.
- [ ] Switch `runie-tui` declarations to `workspace = true`.
- [ ] `cargo check --workspace` passes.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** `cargo check --workspace` passes.
- **Live tmux validation:** TUI starts and uses hoisted deps.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
