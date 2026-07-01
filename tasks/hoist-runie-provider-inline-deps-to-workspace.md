# Hoist runie-provider inline deps to workspace

## Status

`todo`

## Context

`crates/runie-provider/Cargo.toml` pins `async-stream`, `reqwest-eventsource`, and `wiremock` inline.

## Goal

Move them to `[workspace.dependencies]` (dev-only for `wiremock`) and use `workspace = true`.

## Acceptance Criteria
- [ ] Add `async-stream`, `reqwest-eventsource`, and `wiremock` to workspace deps.
- [ ] Use `workspace = true` in `runie-provider/Cargo.toml`.
- [ ] `cargo check -p runie-provider` passes.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** `cargo test -p runie-provider` passes.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
